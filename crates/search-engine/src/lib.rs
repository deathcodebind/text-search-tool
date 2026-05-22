use std::collections::HashMap;

use shared::{AppError, ErrorCode, Record, SearchHit, SearchRequest, SearchResponse};
use storage::StorageRepository;
use tantivy::collector::TopDocs;
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, QueryParser};
use tantivy::schema::{Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Index, IndexReader, TantivyDocument, doc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    pub source_id: String,
    pub title: String,
    pub content: String,
    pub region_code: String,
}

pub trait SearchService {
    fn search(&self, request: &SearchRequest) -> Result<SearchResponse, AppError>;
    fn rebuild_index(&self) -> Result<(), AppError>;
}

pub trait IndexingSearchService: SearchService {
    fn replace_from_records(&mut self, records: &[Record]) -> Result<(), AppError>;
    fn append_records(&mut self, records: &[Record]) -> Result<(), AppError>;
}

#[derive(Debug, Default)]
pub struct InMemorySearchService {
    documents: Vec<SearchDocument>,
}

impl InMemorySearchService {
    pub fn new(documents: Vec<SearchDocument>) -> Self {
        Self { documents }
    }

    fn evaluate_document(&self, doc: &SearchDocument, request: &SearchRequest) -> Option<SearchHit> {
        let rule = &request.rule;
        let text = format!("{}\n{}", doc.title, doc.content);

        for term in &rule.must {
            if !contains_keyword(&text, term) {
                return None;
            }
        }

        for term in &rule.must_not {
            if contains_keyword(&text, term) {
                return None;
            }
        }

        let should_hit_count = rule
            .should
            .iter()
            .filter(|term| contains_keyword(&text, term))
            .count() as u32;

        if should_hit_count < rule.minimum_should_match {
            return None;
        }

        let mut score = 0i64;
        score += (rule.must.len() as i64) * 100;
        score += (should_hit_count as i64) * 10;

        Some(SearchHit {
            source_id: doc.source_id.clone(),
            title: doc.title.clone(),
            region_code: doc.region_code.clone(),
            score,
            snippet: build_snippet(&doc.content, &rule.must, &rule.should),
        })
    }
}

fn contains_keyword(text: &str, keyword: &str) -> bool {
    if keyword.trim().is_empty() {
        return false;
    }
    text.to_lowercase().contains(&keyword.to_lowercase())
}

fn build_snippet(content: &str, must: &[String], should: &[String]) -> String {
    let mut terms: Vec<&String> = must.iter().collect();
    terms.extend(should.iter());

    for term in terms {
        if term.trim().is_empty() {
            continue;
        }

        if contains_keyword(content, term) {
            return content.chars().take(80).collect();
        }
    }

    content.chars().take(48).collect()
}

impl SearchService for InMemorySearchService {
    fn search(&self, request: &SearchRequest) -> Result<SearchResponse, AppError> {
        let mut hits: Vec<SearchHit> = self
            .documents
            .iter()
            .filter_map(|doc| self.evaluate_document(doc, request))
            .collect();

        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.source_id.cmp(&b.source_id)));

        let page = request.page.max(1) as usize;
        let page_size = request.page_size.max(1) as usize;
        let start = (page - 1) * page_size;

        let items = if start >= hits.len() {
            Vec::new()
        } else {
            let end = (start + page_size).min(hits.len());
            hits[start..end].to_vec()
        };

        Ok(SearchResponse {
            total: hits.len() as u64,
            hits: items,
        })
    }

    fn rebuild_index(&self) -> Result<(), AppError> {
        Ok(())
    }
}

impl IndexingSearchService for InMemorySearchService {
    fn replace_from_records(&mut self, records: &[Record]) -> Result<(), AppError> {
        self.documents = records
            .iter()
            .map(search_document_from_record)
            .collect::<Vec<_>>();
        Ok(())
    }

    fn append_records(&mut self, records: &[Record]) -> Result<(), AppError> {
        self.documents
            .extend(records.iter().map(search_document_from_record));
        Ok(())
    }
}

pub fn sync_from_storage<R: StorageRepository, E: IndexingSearchService>(
    repo: &R,
    engine: &mut E,
) -> Result<usize, AppError> {
    let records = repo.list_records()?;
    engine.replace_from_records(&records)?;
    Ok(records.len())
}

pub struct TantivySearchService {
    index: Index,
    reader: IndexReader,
    source_id_field: Field,
    title_field: Field,
    content_field: Field,
    region_code_field: Field,
    documents_by_id: HashMap<String, SearchDocument>,
}

impl TantivySearchService {
    pub fn new_in_memory() -> Result<Self, AppError> {
        let mut schema_builder = Schema::builder();
        let source_id_field = schema_builder.add_text_field("source_id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let region_code_field = schema_builder.add_text_field("region_code", STRING | STORED);
        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema);
        let reader = index.reader().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to create tantivy reader: {err}"),
            )
        })?;

        Ok(Self {
            index,
            reader,
            source_id_field,
            title_field,
            content_field,
            region_code_field,
            documents_by_id: HashMap::new(),
        })
    }

    fn write_records(&mut self, records: &[Record], replace_all: bool) -> Result<(), AppError> {
        let mut writer = self.index.writer(50_000_000).map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to create tantivy writer: {err}"),
            )
        })?;

        if replace_all {
            writer.delete_all_documents().map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to clear tantivy index: {err}"),
                )
            })?;
            self.documents_by_id.clear();
        }

        for record in records {
            let source_id = record.source_id.clone();
            let title = record.title.clone();
            let content = record.title.clone();
            let region_code = record.region_code.clone();

            writer.add_document(doc!(
                self.source_id_field => source_id.clone(),
                self.title_field => title.clone(),
                self.content_field => content.clone(),
                self.region_code_field => region_code.clone(),
            ))
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to add tantivy document: {err}"),
                )
            })?;

            self.documents_by_id.insert(
                source_id.clone(),
                SearchDocument {
                    source_id,
                    title,
                    content,
                    region_code,
                },
            );
        }

        writer.commit().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to commit tantivy index: {err}"),
            )
        })?;
        self.reader.reload().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to reload tantivy reader: {err}"),
            )
        })?;

        Ok(())
    }
}

impl IndexingSearchService for TantivySearchService {
    fn replace_from_records(&mut self, records: &[Record]) -> Result<(), AppError> {
        self.write_records(records, true)
    }

    fn append_records(&mut self, records: &[Record]) -> Result<(), AppError> {
        self.write_records(records, false)
    }
}

impl SearchService for TantivySearchService {
    fn search(&self, request: &SearchRequest) -> Result<SearchResponse, AppError> {
        let searcher = self.reader.searcher();
        let parser = QueryParser::for_index(&self.index, vec![self.title_field, self.content_field]);

        let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for term in &request.rule.must {
            if term.trim().is_empty() {
                continue;
            }
            let q = parser.parse_query(term).map_err(|err| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    format!("failed to parse must query term: {err}"),
                )
            })?;
            clauses.push((Occur::Must, q));
        }

        for term in &request.rule.must_not {
            if term.trim().is_empty() {
                continue;
            }
            let q = parser.parse_query(term).map_err(|err| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    format!("failed to parse must_not query term: {err}"),
                )
            })?;
            clauses.push((Occur::MustNot, q));
        }

        for term in &request.rule.should {
            if term.trim().is_empty() {
                continue;
            }
            let q = parser.parse_query(term).map_err(|err| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    format!("failed to parse should query term: {err}"),
                )
            })?;
            clauses.push((Occur::Should, q));
        }

        let query: Box<dyn Query> = if clauses.is_empty() {
            Box::new(AllQuery)
        } else {
            Box::new(BooleanQuery::from(clauses))
        };

        let mut top_docs = searcher
            .search(&query, &TopDocs::with_limit(5000))
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("tantivy search failed: {err}"),
                )
            })?;

        if top_docs.is_empty() {
            top_docs = searcher
                .search(&AllQuery, &TopDocs::with_limit(5000))
                .map_err(|err| {
                    AppError::new(
                        ErrorCode::Infrastructure,
                        format!("tantivy fallback search failed: {err}"),
                    )
                })?;
        }

        let mut hits: Vec<SearchHit> = Vec::new();
        for (score, addr) in top_docs {
            let doc: TantivyDocument = searcher.doc(addr).map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("failed to read tantivy document: {err}"),
                )
            })?;

            let source_id = match doc
                .get_first(self.source_id_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                Some(v) => v,
                None => continue,
            };

            let Some(stored_doc) = self.documents_by_id.get(&source_id) else {
                continue;
            };

            let text = format!("{}\n{}", stored_doc.title, stored_doc.content);

            let all_must_hit = request
                .rule
                .must
                .iter()
                .all(|term| contains_keyword(&text, term));
            if !all_must_hit {
                continue;
            }

            let has_must_not = request
                .rule
                .must_not
                .iter()
                .any(|term| contains_keyword(&text, term));
            if has_must_not {
                continue;
            }

            let should_hit_count = request
                .rule
                .should
                .iter()
                .filter(|term| contains_keyword(&text, term))
                .count() as u32;

            if should_hit_count < request.rule.minimum_should_match {
                continue;
            }

            let merged_score = (score * 1000.0) as i64 + (should_hit_count as i64) * 10;

            hits.push(SearchHit {
                source_id: stored_doc.source_id.clone(),
                title: stored_doc.title.clone(),
                region_code: stored_doc.region_code.clone(),
                score: merged_score,
                snippet: build_snippet(
                    &stored_doc.content,
                    &request.rule.must,
                    &request.rule.should,
                ),
            });
        }

        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.source_id.cmp(&b.source_id)));

        let page = request.page.max(1) as usize;
        let page_size = request.page_size.max(1) as usize;
        let start = (page - 1) * page_size;
        let items = if start >= hits.len() {
            Vec::new()
        } else {
            let end = (start + page_size).min(hits.len());
            hits[start..end].to_vec()
        };

        Ok(SearchResponse {
            total: hits.len() as u64,
            hits: items,
        })
    }

    fn rebuild_index(&self) -> Result<(), AppError> {
        self.reader.reload().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to reload tantivy reader: {err}"),
            )
        })
    }
}

fn search_document_from_record(record: &Record) -> SearchDocument {
    SearchDocument {
        source_id: record.source_id.clone(),
        title: record.title.clone(),
        content: record.title.clone(),
        region_code: record.region_code.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{InMemorySearchService, SearchDocument, SearchService};
    use super::{IndexingSearchService, TantivySearchService, sync_from_storage};
    use shared::{QueryRule, Record, SearchRequest};
    use storage::SqliteStorageRepository;
    use storage::StorageRepository;
    use tempfile::NamedTempFile;

    fn doc(id: &str, title: &str, content: &str) -> SearchDocument {
        SearchDocument {
            source_id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            region_code: "360103".to_string(),
        }
    }

    #[test]
    fn must_and_must_not_should_filter_documents() {
        let service = InMemorySearchService::new(vec![
            doc("1", "弱电改造项目", "包含安防和综合布线"),
            doc("2", "家具采购项目", "办公桌椅和柜体"),
            doc("3", "弱电维护", "仅巡检，不包含施工"),
        ]);

        let request = SearchRequest {
            rule: QueryRule {
                must: vec!["弱电".to_string()],
                should: vec![],
                must_not: vec!["家具".to_string()],
                minimum_should_match: 0,
            },
            page: 1,
            page_size: 20,
        };

        let response = service.search(&request).expect("search should succeed");
        assert_eq!(response.total, 2);
        assert_eq!(response.hits.len(), 2);
        assert_eq!(response.hits[0].source_id, "1");
        assert_eq!(response.hits[1].source_id, "3");
    }

    #[test]
    fn minimum_should_match_should_work() {
        let service = InMemorySearchService::new(vec![
            doc("1", "弱电改造", "安防 综合布线"),
            doc("2", "弱电巡检", "仅安防"),
            doc("3", "弱电维护", "无相关关键词"),
        ]);

        let request = SearchRequest {
            rule: QueryRule {
                must: vec!["弱电".to_string()],
                should: vec!["安防".to_string(), "综合布线".to_string()],
                must_not: vec![],
                minimum_should_match: 2,
            },
            page: 1,
            page_size: 20,
        };

        let response = service.search(&request).expect("search should succeed");
        assert_eq!(response.total, 1);
        assert_eq!(response.hits[0].source_id, "1");
    }

    #[test]
    fn pagination_should_slice_sorted_hits() {
        let service = InMemorySearchService::new(vec![
            doc("1", "弱电改造", "安防 综合布线"),
            doc("2", "弱电巡检", "安防"),
            doc("3", "弱电维护", "综合布线"),
        ]);

        let request_page_1 = SearchRequest {
            rule: QueryRule {
                must: vec!["弱电".to_string()],
                should: vec!["安防".to_string(), "综合布线".to_string()],
                must_not: vec![],
                minimum_should_match: 0,
            },
            page: 1,
            page_size: 2,
        };

        let request_page_2 = SearchRequest {
            page: 2,
            ..request_page_1.clone()
        };

        let page_1 = service
            .search(&request_page_1)
            .expect("page 1 search should succeed");
        let page_2 = service
            .search(&request_page_2)
            .expect("page 2 search should succeed");

        assert_eq!(page_1.total, 3);
        assert_eq!(page_1.hits.len(), 2);
        assert_eq!(page_2.hits.len(), 1);
    }

    #[test]
    fn tantivy_search_should_work_with_bool_rule() {
        let mut service = TantivySearchService::new_in_memory().expect("service should init");
        let records = vec![
            Record {
                source_id: "1".to_string(),
                source_url: "https://example.local/1".to_string(),
                title: "弱电改造 安防 综合布线".to_string(),
                region_code: "360103".to_string(),
                published_at: 100,
                expires_at: 999,
            },
            Record {
                source_id: "2".to_string(),
                source_url: "https://example.local/2".to_string(),
                title: "家具采购".to_string(),
                region_code: "360103".to_string(),
                published_at: 100,
                expires_at: 999,
            },
        ];
        service
            .replace_from_records(&records)
            .expect("index should be replaced");

        let response = service
            .search(&SearchRequest {
                rule: QueryRule {
                    must: vec!["弱电".to_string()],
                    should: vec!["安防".to_string()],
                    must_not: vec!["家具".to_string()],
                    minimum_should_match: 1,
                },
                page: 1,
                page_size: 10,
            })
            .expect("search should succeed");

        assert_eq!(response.total, 1);
        assert_eq!(response.hits[0].source_id, "1");
    }

    #[test]
    fn sync_from_storage_should_reload_index() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");
        repo.upsert_records(&[
            Record {
                source_id: "1".to_string(),
                source_url: "https://example.local/1".to_string(),
                title: "弱电改造".to_string(),
                region_code: "360103".to_string(),
                published_at: 100,
                expires_at: 999,
            },
            Record {
                source_id: "2".to_string(),
                source_url: "https://example.local/2".to_string(),
                title: "综合布线服务".to_string(),
                region_code: "360103".to_string(),
                published_at: 101,
                expires_at: 999,
            },
        ])
        .expect("upsert should succeed");

        let mut service = TantivySearchService::new_in_memory().expect("service should init");
        let synced = sync_from_storage(&repo, &mut service).expect("sync should succeed");
        assert_eq!(synced, 2);

        let response = service
            .search(&SearchRequest {
                rule: QueryRule {
                    must: vec!["弱电".to_string()],
                    should: vec![],
                    must_not: vec![],
                    minimum_should_match: 0,
                },
                page: 1,
                page_size: 10,
            })
            .expect("search should succeed");

        assert_eq!(response.total, 1);
        assert_eq!(response.hits[0].source_id, "1");
    }
}
