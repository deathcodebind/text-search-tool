use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension, params};
use shared::{AppError, CrawlJobStatus, ErrorCode, QueryRule, Record};

const TERM_SEPARATOR: &str = "\u{1f}";

pub trait StorageRepository {
    fn upsert_records(&self, records: &[Record]) -> Result<u64, AppError>;
    fn associate_job_records(&self, job_id: &str, source_ids: &[String]) -> Result<(), AppError>;
    fn list_records(&self) -> Result<Vec<Record>, AppError>;
    fn mark_job_status(
        &self,
        job_id: &str,
        status: CrawlJobStatus,
        message: Option<&str>,
    ) -> Result<(), AppError>;
    fn delete_expired(&self, now_ts: i64) -> Result<u64, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredKeywordRule {
    pub rule_id: String,
    pub name: String,
    pub enabled: bool,
    pub rule: QueryRule,
    pub updated_at: i64,
}

pub struct SqliteStorageRepository {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRecordDocument {
    pub record: Record,
    pub detail_text: Option<String>,
    pub detail_status: Option<String>,
    pub detail_message: Option<String>,
    pub detail_attempts: u32,
    pub detail_updated_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDetailPayload {
    pub source_id: String,
    pub detail_text: String,
    pub raw_json: String,
    pub source_page_url: String,
    pub attachment_urls: Vec<String>,
}

impl SqliteStorageRepository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to open sqlite database: {err}"),
            )
        })?;

        let repo = Self {
            conn: Mutex::new(conn),
        };
        repo.init_schema()?;
        Ok(repo)
    }

    fn init_schema(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS records (
                source_id TEXT PRIMARY KEY,
                source_url TEXT NOT NULL,
                title TEXT NOT NULL,
                region_code TEXT NOT NULL,
                published_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_records_source_url_non_empty
            ON records(source_url)
            WHERE source_url <> '';

            CREATE INDEX IF NOT EXISTS idx_records_expires_at ON records(expires_at);

            CREATE TABLE IF NOT EXISTS crawl_jobs (
                job_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                message TEXT,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS crawl_job_records (
                job_id TEXT NOT NULL,
                source_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (job_id, source_id),
                FOREIGN KEY(job_id) REFERENCES crawl_jobs(job_id) ON DELETE CASCADE,
                FOREIGN KEY(source_id) REFERENCES records(source_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_crawl_job_records_job_id
            ON crawl_job_records(job_id, created_at DESC, source_id ASC);

            CREATE TABLE IF NOT EXISTS keyword_rules (
                rule_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                must_terms TEXT NOT NULL,
                should_terms TEXT NOT NULL,
                must_not_terms TEXT NOT NULL,
                minimum_should_match INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS record_details (
                source_id TEXT PRIMARY KEY,
                detail_text TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(source_id) REFERENCES records(source_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS record_detail_meta (
                source_id TEXT PRIMARY KEY,
                raw_json TEXT NOT NULL,
                source_page_url TEXT NOT NULL,
                attachment_urls TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(source_id) REFERENCES records(source_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS record_detail_jobs (
                source_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                message TEXT,
                attempts INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY(source_id) REFERENCES records(source_id) ON DELETE CASCADE
            );
            "#,
        )
        .map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to initialize sqlite schema: {err}"),
            )
        })?;

        Ok(())
    }

    fn now_ts() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    fn status_to_string(status: &CrawlJobStatus) -> &'static str {
        match status {
            CrawlJobStatus::Pending => "pending",
            CrawlJobStatus::Running => "running",
            CrawlJobStatus::Succeeded => "succeeded",
            CrawlJobStatus::Failed => "failed",
        }
    }

    pub fn get_job_status(&self, job_id: &str) -> Result<Option<CrawlJobStatus>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let status: Option<String> = conn
            .query_row(
                "SELECT status FROM crawl_jobs WHERE job_id = ?1",
                params![job_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query job status: {err}"),
                )
            })?;

        let mapped = status.as_deref().map(|x| match x {
            "pending" => CrawlJobStatus::Pending,
            "running" => CrawlJobStatus::Running,
            "succeeded" => CrawlJobStatus::Succeeded,
            _ => CrawlJobStatus::Failed,
        });

        Ok(mapped)
    }

    pub fn get_job_message(&self, job_id: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        conn
            .query_row(
                "SELECT message FROM crawl_jobs WHERE job_id = ?1",
                params![job_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(|value| value.flatten())
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query job message: {err}"),
                )
            })
    }

    pub fn count_job_records(&self, job_id: &str) -> Result<u64, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let cnt: i64 = conn
            .query_row(
                "SELECT COUNT(1) FROM crawl_job_records WHERE job_id = ?1",
                params![job_id],
                |row| row.get(0),
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to count job records: {err}"),
                )
            })?;

        Ok(cnt as u64)
    }

    pub fn count_records(&self) -> Result<u64, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let cnt: i64 = conn
            .query_row("SELECT COUNT(1) FROM records", [], |row| row.get(0))
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to count records: {err}"),
                )
            })?;

        Ok(cnt as u64)
    }

    pub fn upsert_keyword_rule(
        &self,
        rule_id: &str,
        name: &str,
        enabled: bool,
        rule: &QueryRule,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        conn.execute(
            r#"
            INSERT INTO keyword_rules (
                rule_id,
                name,
                enabled,
                must_terms,
                should_terms,
                must_not_terms,
                minimum_should_match,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(rule_id) DO UPDATE SET
                name=excluded.name,
                enabled=excluded.enabled,
                must_terms=excluded.must_terms,
                should_terms=excluded.should_terms,
                must_not_terms=excluded.must_not_terms,
                minimum_should_match=excluded.minimum_should_match,
                updated_at=excluded.updated_at
            "#,
            params![
                rule_id,
                name,
                if enabled { 1 } else { 0 },
                encode_terms(&rule.must),
                encode_terms(&rule.should),
                encode_terms(&rule.must_not),
                rule.minimum_should_match as i64,
                Self::now_ts(),
            ],
        )
        .map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to upsert keyword rule: {err}"),
            )
        })?;

        Ok(())
    }

    pub fn list_keyword_rules(&self) -> Result<Vec<StoredKeywordRule>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    rule_id,
                    name,
                    enabled,
                    must_terms,
                    should_terms,
                    must_not_terms,
                    minimum_should_match,
                    updated_at
                FROM keyword_rules
                ORDER BY updated_at DESC, rule_id ASC
                "#,
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to prepare keyword rule list query: {err}"),
                )
            })?;

        let rows = stmt
            .query_map([], |row| {
                Ok(StoredKeywordRule {
                    rule_id: row.get(0)?,
                    name: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    rule: QueryRule {
                        must: decode_terms(&row.get::<_, String>(3)?),
                        should: decode_terms(&row.get::<_, String>(4)?),
                        must_not: decode_terms(&row.get::<_, String>(5)?),
                        minimum_should_match: row.get::<_, i64>(6)? as u32,
                    },
                    updated_at: row.get(7)?,
                })
            })
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query keyword rules: {err}"),
                )
            })?;

        let mut rules = Vec::new();
        for row in rows {
            rules.push(row.map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to map keyword rule row: {err}"),
                )
            })?);
        }

        Ok(rules)
    }

    pub fn delete_keyword_rule(&self, rule_id: &str) -> Result<bool, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let affected = conn
            .execute("DELETE FROM keyword_rules WHERE rule_id = ?1", params![rule_id])
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to delete keyword rule: {err}"),
                )
            })?;

        Ok(affected > 0)
    }

    pub fn upsert_record_details(&self, details: &[(String, String)]) -> Result<u64, AppError> {
        let mut conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let tx = conn.transaction().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to start sqlite transaction: {err}"),
            )
        })?;

        for (source_id, detail_text) in details {
            tx.execute(
                r#"
                INSERT INTO record_details (source_id, detail_text, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(source_id) DO UPDATE SET
                    detail_text=excluded.detail_text,
                    updated_at=excluded.updated_at
                "#,
                params![source_id, detail_text, Self::now_ts()],
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to upsert detail for {}: {err}", source_id),
                )
            })?;
        }

        tx.commit().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to commit sqlite transaction: {err}"),
            )
        })?;

        Ok(details.len() as u64)
    }

    pub fn list_record_documents(&self) -> Result<Vec<StoredRecordDocument>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    r.source_id,
                    r.source_url,
                    r.title,
                    r.region_code,
                    r.published_at,
                    r.expires_at,
                    d.detail_text,
                    j.status,
                    j.message,
                    j.attempts,
                    j.updated_at
                FROM records r
                LEFT JOIN record_details d ON d.source_id = r.source_id
                LEFT JOIN record_detail_jobs j ON j.source_id = r.source_id
                ORDER BY r.published_at DESC, r.source_id ASC
                "#,
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to prepare document list query: {err}"),
                )
            })?;

        let rows = stmt
            .query_map([], |row| {
                Ok(StoredRecordDocument {
                    record: Record {
                        source_id: row.get(0)?,
                        source_url: row.get(1)?,
                        title: row.get(2)?,
                        region_code: row.get(3)?,
                        published_at: row.get(4)?,
                        expires_at: row.get(5)?,
                    },
                    detail_text: row.get(6)?,
                    detail_status: row.get(7)?,
                    detail_message: row.get(8)?,
                    detail_attempts: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as u32,
                    detail_updated_at: row.get(10)?,
                })
            })
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query record documents: {err}"),
                )
            })?;

        let mut docs = Vec::new();
        for row in rows {
            docs.push(row.map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to map record document row: {err}"),
                )
            })?);
        }

        Ok(docs)
    }

    pub fn list_record_documents_by_job(
        &self,
        job_id: &str,
    ) -> Result<Vec<StoredRecordDocument>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    r.source_id,
                    r.source_url,
                    r.title,
                    r.region_code,
                    r.published_at,
                    r.expires_at,
                    d.detail_text,
                    j.status,
                    j.message,
                    j.attempts,
                    j.updated_at
                FROM crawl_job_records cjr
                INNER JOIN records r ON r.source_id = cjr.source_id
                LEFT JOIN record_details d ON d.source_id = r.source_id
                LEFT JOIN record_detail_jobs j ON j.source_id = r.source_id
                WHERE cjr.job_id = ?1
                ORDER BY r.published_at DESC, r.source_id ASC
                "#,
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to prepare job scoped document list query: {err}"),
                )
            })?;

        let rows = stmt
            .query_map(params![job_id], |row| {
                Ok(StoredRecordDocument {
                    record: Record {
                        source_id: row.get(0)?,
                        source_url: row.get(1)?,
                        title: row.get(2)?,
                        region_code: row.get(3)?,
                        published_at: row.get(4)?,
                        expires_at: row.get(5)?,
                    },
                    detail_text: row.get(6)?,
                    detail_status: row.get(7)?,
                    detail_message: row.get(8)?,
                    detail_attempts: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as u32,
                    detail_updated_at: row.get(10)?,
                })
            })
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query job scoped record documents: {err}"),
                )
            })?;

        let mut docs = Vec::new();
        for row in rows {
            docs.push(row.map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to map job scoped record document row: {err}"),
                )
            })?);
        }

        Ok(docs)
    }

    pub fn upsert_record_detail_payloads(
        &self,
        payloads: &[RecordDetailPayload],
    ) -> Result<u64, AppError> {
        let mut conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let tx = conn.transaction().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to start sqlite transaction: {err}"),
            )
        })?;

        for payload in payloads {
            tx.execute(
                r#"
                INSERT INTO record_details (source_id, detail_text, updated_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(source_id) DO UPDATE SET
                    detail_text=excluded.detail_text,
                    updated_at=excluded.updated_at
                "#,
                params![payload.source_id, payload.detail_text, Self::now_ts()],
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to upsert detail text for {}: {err}", payload.source_id),
                )
            })?;

            tx.execute(
                r#"
                INSERT INTO record_detail_meta (
                    source_id,
                    raw_json,
                    source_page_url,
                    attachment_urls,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(source_id) DO UPDATE SET
                    raw_json=excluded.raw_json,
                    source_page_url=excluded.source_page_url,
                    attachment_urls=excluded.attachment_urls,
                    updated_at=excluded.updated_at
                "#,
                params![
                    payload.source_id,
                    payload.raw_json,
                    payload.source_page_url,
                    serde_json::to_string(&payload.attachment_urls).map_err(|err| {
                        AppError::new(
                            ErrorCode::Infrastructure,
                            format!("failed to serialize attachment urls: {err}"),
                        )
                    })?,
                    Self::now_ts()
                ],
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to upsert detail meta for {}: {err}", payload.source_id),
                )
            })?;
        }

        tx.commit().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to commit sqlite transaction: {err}"),
            )
        })?;

        Ok(payloads.len() as u64)
    }

    pub fn get_record_detail_payload(
        &self,
        source_id: &str,
    ) -> Result<Option<RecordDetailPayload>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT
                    d.source_id,
                    d.detail_text,
                    m.raw_json,
                    m.source_page_url,
                    m.attachment_urls
                FROM record_details d
                LEFT JOIN record_detail_meta m ON m.source_id = d.source_id
                WHERE d.source_id = ?1
                "#,
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to prepare detail payload query: {err}"),
                )
            })?;

        let row = stmt
            .query_row(params![source_id], |row| {
                let attachment_raw: Option<String> = row.get(4)?;
                let attachment_urls = if let Some(raw) = attachment_raw {
                    serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
                } else {
                    Vec::new()
                };

                Ok(RecordDetailPayload {
                    source_id: row.get(0)?,
                    detail_text: row.get(1)?,
                    raw_json: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    source_page_url: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    attachment_urls,
                })
            })
            .optional()
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query detail payload: {err}"),
                )
            })?;

        Ok(row)
    }

    pub fn upsert_record_detail_job_status(
        &self,
        source_id: &str,
        status: &str,
        message: Option<&str>,
        attempts: u32,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        conn.execute(
            r#"
            INSERT INTO record_detail_jobs (source_id, status, message, attempts, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(source_id) DO UPDATE SET
                status=excluded.status,
                message=excluded.message,
                attempts=excluded.attempts,
                updated_at=excluded.updated_at
            "#,
            params![source_id, status, message.unwrap_or(""), attempts as i64, Self::now_ts()],
        )
        .map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to upsert detail job status for {}: {err}", source_id),
            )
        })?;

        Ok(())
    }

    pub fn get_record_by_source_id(&self, source_id: &str) -> Result<Option<Record>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        conn
            .query_row(
                r#"
                SELECT source_id, source_url, title, region_code, published_at, expires_at
                FROM records
                WHERE source_id = ?1
                "#,
                params![source_id],
                |row| {
                    Ok(Record {
                        source_id: row.get(0)?,
                        source_url: row.get(1)?,
                        title: row.get(2)?,
                        region_code: row.get(3)?,
                        published_at: row.get(4)?,
                        expires_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query record by source id: {err}"),
                )
            })
    }
}

fn encode_terms(terms: &[String]) -> String {
    terms.join(TERM_SEPARATOR)
}

fn decode_terms(encoded: &str) -> Vec<String> {
    if encoded.is_empty() {
        return Vec::new();
    }
    encoded.split(TERM_SEPARATOR).map(|s| s.to_string()).collect()
}

impl StorageRepository for SqliteStorageRepository {
    fn upsert_records(&self, records: &[Record]) -> Result<u64, AppError> {
        let mut conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let tx = conn.transaction().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to start sqlite transaction: {err}"),
            )
        })?;

        for record in records {
            tx.execute(
                r#"
                INSERT INTO records (
                    source_id,
                    source_url,
                    title,
                    region_code,
                    published_at,
                    expires_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(source_id) DO UPDATE SET
                    source_url=excluded.source_url,
                    title=excluded.title,
                    region_code=excluded.region_code,
                    published_at=excluded.published_at,
                    expires_at=excluded.expires_at
                "#,
                params![
                    record.source_id,
                    record.source_url,
                    record.title,
                    record.region_code,
                    record.published_at,
                    record.expires_at,
                ],
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to upsert record {}: {err}", record.source_id),
                )
            })?;
        }

        tx.commit().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to commit sqlite transaction: {err}"),
            )
        })?;

        Ok(records.len() as u64)
    }

    fn associate_job_records(&self, job_id: &str, source_ids: &[String]) -> Result<(), AppError> {
        let mut conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let tx = conn.transaction().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to start sqlite transaction: {err}"),
            )
        })?;

        for source_id in source_ids {
            tx.execute(
                r#"
                INSERT INTO crawl_job_records (job_id, source_id, created_at)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(job_id, source_id) DO NOTHING
                "#,
                params![job_id, source_id, Self::now_ts()],
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!(
                        "failed to associate record {} with job {}: {err}",
                        source_id, job_id
                    ),
                )
            })?;
        }

        tx.commit().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to commit sqlite transaction: {err}"),
            )
        })?;

        Ok(())
    }

    fn list_records(&self) -> Result<Vec<Record>, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT source_id, source_url, title, region_code, published_at, expires_at
                FROM records
                ORDER BY published_at DESC, source_id ASC
                "#,
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to prepare list query: {err}"),
                )
            })?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Record {
                    source_id: row.get(0)?,
                    source_url: row.get(1)?,
                    title: row.get(2)?,
                    region_code: row.get(3)?,
                    published_at: row.get(4)?,
                    expires_at: row.get(5)?,
                })
            })
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to query records: {err}"),
                )
            })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to map record row: {err}"),
                )
            })?);
        }

        Ok(records)
    }

    fn mark_job_status(
        &self,
        job_id: &str,
        status: CrawlJobStatus,
        message: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        conn.execute(
            r#"
            INSERT INTO crawl_jobs (job_id, status, message, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(job_id) DO UPDATE SET
                status=excluded.status,
                message=excluded.message,
                updated_at=excluded.updated_at
            "#,
            params![
                job_id,
                Self::status_to_string(&status),
                message.unwrap_or(""),
                Self::now_ts(),
            ],
        )
        .map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to upsert crawl job status: {err}"),
            )
        })?;

        Ok(())
    }

    fn delete_expired(&self, now_ts: i64) -> Result<u64, AppError> {
        let conn = self.conn.lock().map_err(|_| {
            AppError::new(ErrorCode::Infrastructure, "failed to acquire sqlite lock")
        })?;

        let affected = conn
            .execute(
                "DELETE FROM records WHERE expires_at <= ?1",
                params![now_ts],
            )
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to delete expired records: {err}"),
                )
            })?;

        Ok(affected as u64)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::{SqliteStorageRepository, StorageRepository};
    use shared::{CrawlJobStatus, QueryRule, Record};

    fn record(
        source_id: &str,
        source_url: &str,
        title: &str,
        region_code: &str,
        published_at: i64,
        expires_at: i64,
    ) -> Record {
        Record {
            source_id: source_id.to_string(),
            source_url: source_url.to_string(),
            title: title.to_string(),
            region_code: region_code.to_string(),
            published_at,
            expires_at,
        }
    }

    #[test]
    fn upsert_should_deduplicate_by_source_id() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");

        repo.upsert_records(&[
            record("rid-1", "https://a.example/1", "t1", "360103", 100, 200),
            record("rid-1", "https://a.example/1", "t1-updated", "360104", 110, 210),
        ])
        .expect("upsert should succeed");

        let count = repo.count_records().expect("count should succeed");
        assert_eq!(count, 1);

        let all = repo.list_records().expect("list should succeed");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].title, "t1-updated");
    }

    #[test]
    fn delete_expired_should_only_remove_expired_rows() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");

        repo.upsert_records(&[
            record("rid-1", "https://a.example/1", "t1", "360103", 100, 200),
            record("rid-2", "https://a.example/2", "t2", "360104", 100, 999),
        ])
        .expect("upsert should succeed");

        let deleted = repo.delete_expired(500).expect("delete should succeed");
        assert_eq!(deleted, 1);
        assert_eq!(repo.count_records().expect("count should succeed"), 1);
    }

    #[test]
    fn mark_job_status_should_upsert_job_row() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");

        repo.mark_job_status("job-1", CrawlJobStatus::Running, Some("in progress"))
            .expect("job status should be inserted");
        repo.mark_job_status("job-1", CrawlJobStatus::Succeeded, Some("done"))
            .expect("job status should be updated");

        let status = repo
            .get_job_status("job-1")
            .expect("query status should succeed");
        assert_eq!(status, Some(CrawlJobStatus::Succeeded));
    }

    #[test]
    fn keyword_rules_should_support_upsert_list_and_delete() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");

        let rule = QueryRule {
            must: vec!["弱电".to_string()],
            should: vec!["安防".to_string()],
            must_not: vec!["家具".to_string()],
            minimum_should_match: 1,
        };

        repo.upsert_keyword_rule("rule-1", "弱电规则", true, &rule)
            .expect("rule upsert should succeed");

        let listed = repo
            .list_keyword_rules()
            .expect("rule list should succeed");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].rule_id, "rule-1");
        assert_eq!(listed[0].rule.must, vec!["弱电"]);
        assert_eq!(listed[0].rule.minimum_should_match, 1);

        let deleted = repo
            .delete_keyword_rule("rule-1")
            .expect("rule delete should succeed");
        assert!(deleted);
        assert!(repo
            .list_keyword_rules()
            .expect("list after delete should succeed")
            .is_empty());
    }

    #[test]
    fn record_documents_should_include_detail_text() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");

        repo.upsert_records(&[record(
            "rid-1",
            "https://a.example/1",
            "t1",
            "360103",
            100,
            200,
        )])
        .expect("upsert should succeed");

        repo
            .upsert_record_details(&[(
                "rid-1".to_string(),
                "这是详情页正文内容".to_string(),
            )])
            .expect("detail upsert should succeed");

        let docs = repo
            .list_record_documents()
            .expect("document list should succeed");
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].record.source_id, "rid-1");
        assert_eq!(docs[0].detail_text.as_deref(), Some("这是详情页正文内容"));
    }
}
