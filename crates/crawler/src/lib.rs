use shared::{AppError, CrawlJobProgress, CrawlJobRequest, ErrorCode, Record};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, ORIGIN, REFERER};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use storage::StorageRepository;

pub mod login;

pub use login::{
    JxemallLoginHttpClient, JxemallLoginInput, JxemallLoginResponse, extract_cookie_value,
};

pub fn persist_crawl_batch<R: StorageRepository>(
    repo: &R,
    job_id: &str,
    records: &[Record],
) -> Result<u64, AppError> {
    repo.mark_job_status(job_id, shared::CrawlJobStatus::Running, Some("ingesting records"))?;

    let upserted = match repo.upsert_records(records) {
        Ok(value) => value,
        Err(err) => {
            let err_message = err.message.clone();
            let _ = repo.mark_job_status(
                job_id,
                shared::CrawlJobStatus::Failed,
                Some(err_message.as_str()),
            );
            return Err(err);
        }
    };

    repo.mark_job_status(job_id, shared::CrawlJobStatus::Succeeded, Some("ingest completed"))?;
    Ok(upserted)
}

pub const JXEMALL_LOGIN_PATH: &str = "/login";
pub const JXEMALL_LOGIN_QUERY_CURRENT_URI: &str =
    "current_uri=https%3A%2F%2Flogin.jxemall.com%2Fuser-login%2F%23%2F";
pub const JXEMALL_LOGIN_REFERER: &str = "https://login.jxemall.com/user-login/";
pub const JXEMALL_LOGIN_ORIGIN: &str = "https://login.jxemall.com";
pub const JXEMALL_LIST_NEWEST_PATH: &str = "/api/sparta/announcement/listNewest";
pub const JXEMALL_LIST_NEWEST_ORIGIN: &str = "https://www.jxemall.com";
pub const JXEMALL_LIST_NEWEST_REFERER_BIDDING: &str =
    "https://www.jxemall.com/luban/bidding/newest?tradeModel=BIDDING&tradeStyle=BIDDING";
pub const JXEMALL_DETAIL_PATH: &str = "/api/sparta/announcement/detail";
pub const JXEMALL_DETAIL_REFERER_TEMPLATE: &str =
    "https://www.jxemall.com/luban/bidding/detail?requisitionId={requisitionId}&type={type}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JxemallLoginFormInput {
    pub username: String,
    pub password_value: String,
}

pub fn build_jxemall_login_form_fields(
    input: &JxemallLoginFormInput,
) -> Result<Vec<(String, String)>, AppError> {
    let username = input.username.trim();
    let password_value = input.password_value.trim();

    if username.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "username is required",
        ));
    }

    if password_value.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "password value is required",
        ));
    }

    Ok(vec![
        ("platformCode".to_string(), "zcy".to_string()),
        ("loginType".to_string(), "password".to_string()),
        ("requestType".to_string(), "async".to_string()),
        ("username".to_string(), username.to_string()),
        ("password".to_string(), password_value.to_string()),
    ])
}

pub trait CrawlerService {
    fn start_crawl(&self, request: &CrawlJobRequest) -> Result<String, AppError>;
    fn get_progress(&self, job_id: &str) -> Result<CrawlJobProgress, AppError>;
    fn fetch_incremental(&self, job_id: &str) -> Result<Vec<Record>, AppError>;
}

#[derive(Debug, Default)]
pub struct CrawlerServicePlaceholder;

impl CrawlerService for CrawlerServicePlaceholder {
    fn start_crawl(&self, _request: &CrawlJobRequest) -> Result<String, AppError> {
        Ok("job-placeholder".to_string())
    }

    fn get_progress(&self, _job_id: &str) -> Result<CrawlJobProgress, AppError> {
        Ok(CrawlJobProgress {
            job_id: "job-placeholder".to_string(),
            status: shared::CrawlJobStatus::Pending,
            processed: 0,
            total: None,
            message: Some("not started".to_string()),
        })
    }

    fn fetch_incremental(&self, _job_id: &str) -> Result<Vec<Record>, AppError> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListNewestPayload {
    pub district_code_list: Vec<String>,
    pub administrative_district_code_list: Vec<String>,
}

pub fn build_district_code_list_from_selection(
    all_options_in_ui_order: &[String],
    selected_codes: &HashSet<String>,
) -> Result<Vec<String>, AppError> {
    let mut option_set: HashSet<&str> = HashSet::new();
    for code in all_options_in_ui_order {
        if !option_set.insert(code.as_str()) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("duplicate district option found in ui list: {code}"),
            ));
        }
    }

    for code in selected_codes {
        if !option_set.contains(code.as_str()) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("selected district code is not in ui options: {code}"),
            ));
        }
    }

    Ok(all_options_in_ui_order
        .iter()
        .filter(|code| selected_codes.contains(code.as_str()))
        .cloned()
        .collect())
}

pub fn build_list_newest_payload_from_selection(
    all_options_in_ui_order: &[String],
    selected_codes: &HashSet<String>,
    valid_code_set: &HashSet<String>,
) -> Result<ListNewestPayload, AppError> {
    let district_codes = build_district_code_list_from_selection(all_options_in_ui_order, selected_codes)?;
    shared::validate_district_codes(&district_codes, valid_code_set)?;

    Ok(ListNewestPayload {
        district_code_list: district_codes,
        administrative_district_code_list: Vec::new(),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JxemallListNewestRequest {
    pub back_category_name: String,
    pub trade_model: String,
    pub category_type: Option<String>,
    pub page_no: u32,
    pub page_size: u32,
    pub state_list: Vec<u32>,
    pub other_search: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_budget: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_budget: Option<u64>,
    pub instance_codes: Vec<String>,
    pub sort_field: String,
    pub sort_method: String,
    pub district_code_list: Vec<String>,
    pub administrative_district_code_list: Vec<String>,
}

pub struct JxemallListNewestHttpClient {
    client: Client,
    base_url: String,
    cookie_header: String,
}

#[derive(Debug, Clone)]
pub struct JxemallAnnouncementSummary {
    pub record: Record,
    pub requisition_id: String,
    pub announcement_type: String,
}

#[derive(Debug, Clone)]
pub struct JxemallDetailData {
    pub detail_text: String,
    pub raw_json: String,
    pub source_page_url: String,
    pub attachment_urls: Vec<String>,
}

impl JxemallListNewestHttpClient {
    pub fn new(base_url: impl Into<String>, cookie_header: impl Into<String>) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to build http client: {err}"),
            )
        })?;

        Ok(Self {
            client,
            base_url: base_url.into(),
            cookie_header: cookie_header.into(),
        })
    }

    pub fn list_newest(&self, payload: &JxemallListNewestRequest) -> Result<Vec<Record>, AppError> {
        Ok(self
            .list_newest_summaries(payload)?
            .into_iter()
            .map(|x| x.record)
            .collect())
    }

    pub fn list_newest_summaries(
        &self,
        payload: &JxemallListNewestRequest,
    ) -> Result<Vec<JxemallAnnouncementSummary>, AppError> {
        let url = format!(
            "{base}{path}",
            base = self.base_url.trim_end_matches('/'),
            path = JXEMALL_LIST_NEWEST_PATH,
        );

        let response = self
            .client
            .post(url)
            .header(ACCEPT, "application/json, text/plain, */*")
            .header(CONTENT_TYPE, "application/json;charset=UTF-8")
            .header(ORIGIN, JXEMALL_LIST_NEWEST_ORIGIN)
            .header(REFERER, JXEMALL_LIST_NEWEST_REFERER_BIDDING)
            .header("X-Requested-With", "XMLHttpRequest")
            .header(COOKIE, self.cookie_header.as_str())
            .json(payload)
            .send()
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("listNewest request failed: {err}"),
                )
            })?;

        let status = response.status();
        let body: ListNewestResponseBody = response.json().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to parse listNewest response json: {err}"),
            )
        })?;

        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::Infrastructure,
                format!(
                    "listNewest failed with http status {status}: {}",
                    body.message.unwrap_or_else(|| "unknown error".to_string())
                ),
            ));
        }

        if !body.success {
            return Err(AppError::new(
                ErrorCode::Infrastructure,
                format!(
                    "listNewest rejected: {}",
                    body.message.unwrap_or_else(|| "unknown error".to_string())
                ),
            ));
        }

        let items = body
            .result
            .map(|r| r.list)
            .unwrap_or_default();

        Ok(items
            .into_iter()
            .map(|item| {
                let now_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);

                let requisition_id = item
                    .requisition_id
                    .clone()
                    .unwrap_or_else(|| format!("bidding-{}", item.bidding_id.unwrap_or_default()));
                let announcement_type = item
                    .announcement_type
                    .unwrap_or_else(|| "BIDDING_INVITATION".to_string());

                let source_url = format!(
                    "{base}{path}?requisitionId={}&type={}&timestamp={}",
                    requisition_id,
                    announcement_type,
                    now_ms,
                    base = self.base_url.trim_end_matches('/'),
                    path = JXEMALL_DETAIL_PATH,
                );

                JxemallAnnouncementSummary {
                    record: Record {
                        source_id: requisition_id.clone(),
                        source_url,
                        title: item.title,
                        region_code: item.district_code.unwrap_or_else(|| "unknown".to_string()),
                        published_at: item.pub_timestamp / 1000,
                        expires_at: item
                            .end_timestamp
                            .map(|x| x / 1000)
                            .unwrap_or(0),
                    },
                    requisition_id,
                    announcement_type,
                }
            })
            .collect())
    }

    pub fn fetch_detail_data(
        &self,
        requisition_id: &str,
        announcement_type: &str,
    ) -> Result<JxemallDetailData, AppError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        let referer = JXEMALL_DETAIL_REFERER_TEMPLATE
            .replace("{requisitionId}", requisition_id)
            .replace("{type}", announcement_type);

        let url = format!(
            "{base}{path}",
            base = self.base_url.trim_end_matches('/'),
            path = JXEMALL_DETAIL_PATH,
        );

        let response = self
            .client
            .get(url)
            .header(ACCEPT, "application/json, text/plain, */*")
            .header(ORIGIN, JXEMALL_LIST_NEWEST_ORIGIN)
            .header(REFERER, referer)
            .header("X-Requested-With", "XMLHttpRequest")
            .header(COOKIE, self.cookie_header.as_str())
            .query(&[
                ("requisitionId", requisition_id),
                ("type", announcement_type),
                ("timestamp", timestamp.as_str()),
            ])
            .send()
            .map_err(|err| {
                AppError::new(
                    ErrorCode::Infrastructure,
                    format!("detail request failed: {err}"),
                )
            })?;

        let status = response.status();
        let body: Value = response.json().map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to parse detail response json: {err}"),
            )
        })?;

        if !status.is_success() {
            return Err(AppError::new(
                ErrorCode::Infrastructure,
                format!("detail failed with http status {status}"),
            ));
        }

        let success = body
            .get("success")
            .and_then(|x| x.as_bool())
            .unwrap_or(true);
        if !success {
            let message = body
                .get("message")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown detail error");
            return Err(AppError::new(
                ErrorCode::Infrastructure,
                format!("detail rejected: {message}"),
            ));
        }

        let mut fragments = Vec::new();
        collect_text_fragments(body.get("result").unwrap_or(&body), &mut fragments);
        let merged = fragments.join(" ");
        let normalized = merged.split_whitespace().collect::<Vec<_>>().join(" ");

        let raw_json = serde_json::to_string(&body).map_err(|err| {
            AppError::new(
                ErrorCode::Infrastructure,
                format!("failed to serialize detail response json: {err}"),
            )
        })?;

        let source_page_url = JXEMALL_DETAIL_REFERER_TEMPLATE
            .replace("{requisitionId}", requisition_id)
            .replace("{type}", announcement_type);

        let mut attachment_urls = Vec::new();
        collect_attachment_urls(body.get("result").unwrap_or(&body), &mut attachment_urls);
        attachment_urls.sort();
        attachment_urls.dedup();

        Ok(JxemallDetailData {
            detail_text: normalized,
            raw_json,
            source_page_url,
            attachment_urls,
        })
    }
}

fn collect_text_fragments(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.chars().count() >= 2 {
                out.push(trimmed.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_text_fragments(item, out);
            }
        }
        Value::Object(map) => {
            for (_, child) in map {
                collect_text_fragments(child, out);
            }
        }
        _ => {}
    }
}

fn collect_attachment_urls(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let k = key.to_ascii_lowercase();
                if k.contains("url") || k.contains("attachment") || k.contains("file") {
                    if let Some(url) = child.as_str() {
                        let trimmed = url.trim();
                        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                            out.push(trimmed.to_string());
                        }
                    }
                }
                collect_attachment_urls(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_attachment_urls(item, out);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Deserialize)]
struct ListNewestResponseBody {
    success: bool,
    message: Option<String>,
    result: Option<ListNewestResult>,
}

#[derive(Debug, Deserialize)]
struct ListNewestResult {
    #[serde(default)]
    list: Vec<ListNewestItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListNewestItem {
    requisition_id: Option<String>,
    bidding_id: Option<u64>,
    title: String,
    district_code: Option<String>,
    pub_timestamp: i64,
    end_timestamp: Option<i64>,
    #[serde(rename = "type")]
    announcement_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use tempfile::NamedTempFile;

    use shared::Record;
    use storage::SqliteStorageRepository;
    use storage::StorageRepository;

    use crate::{
        build_district_code_list_from_selection, build_jxemall_login_form_fields,
        build_list_newest_payload_from_selection, JxemallLoginFormInput,
        JXEMALL_LOGIN_QUERY_CURRENT_URI, persist_crawl_batch,
    };

    fn valid_set() -> HashSet<String> {
        [
            "360102", "360103", "360104", "360199", "360321", "360322", "360592", "360599",
            "360622", "360699", "360892", "360899", "360992", "361099", "361192", "361199",
            "361209", "369900", "980701",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn to_codes(codes: &[&str]) -> Vec<String> {
        codes.iter().map(|x| x.to_string()).collect()
    }

    fn to_set(codes: &[&str]) -> HashSet<String> {
        codes.iter().map(|x| x.to_string()).collect()
    }

    fn ui_options_for_tests() -> Vec<String> {
        to_codes(&[
            "360102", "360103", "360104", "360199", "360321", "360322", "360592", "360599",
            "360622", "360699", "360892", "360899", "360992", "361099", "361192", "361199",
            "361209", "369900", "980701",
        ])
    }

    #[test]
    fn selected_single_option_should_generate_exact_one_code() {
        let list = build_district_code_list_from_selection(
            &ui_options_for_tests(),
            &to_set(&["360699"]),
        )
        .expect("list should be built from single selected option");

        assert_eq!(list, to_codes(&["360699"]));
    }

    #[test]
    fn selected_cross_city_options_should_generate_stable_ordered_list() {
        let list = build_district_code_list_from_selection(
            &ui_options_for_tests(),
            &to_set(&["361199", "360103", "360321", "361192"]),
        )
        .expect("list should be built from selected options");

        assert_eq!(list, to_codes(&["360103", "360321", "361192", "361199"]));
    }

    #[test]
    fn payload_should_use_generated_district_list_and_keep_administrative_empty() {
        let payload = build_list_newest_payload_from_selection(
            &ui_options_for_tests(),
            &to_set(&["360103", "360321", "361192", "361199"]),
            &valid_set(),
        )
        .expect("payload should be built");

        assert_eq!(payload.district_code_list, to_codes(&["360103", "360321", "361192", "361199"]));
        assert!(payload.administrative_district_code_list.is_empty());
    }

    #[test]
    fn selecting_unknown_option_should_fail() {
        let err = build_district_code_list_from_selection(&ui_options_for_tests(), &to_set(&["369998"]))
            .expect_err("unknown selected option should fail");
        assert!(err.message.contains("not in ui options"));
    }

    #[test]
    fn duplicate_option_in_ui_should_fail() {
        let duplicated_ui = to_codes(&["360103", "360321", "360103"]);
        let err = build_district_code_list_from_selection(&duplicated_ui, &to_set(&["360103"]))
            .expect_err("duplicated ui option should fail");
        assert!(err.message.contains("duplicate district option"));
    }

    #[test]
    fn payload_should_fail_for_invalid_selected_code_format() {
        let ui = to_codes(&["360103", "36A103"]);
        let selected = to_set(&["36A103"]);
        let valid = to_set(&["360103", "36A103"]);
        let err = build_list_newest_payload_from_selection(&ui, &selected, &valid)
            .expect_err("invalid format should fail at payload building");
        assert!(err.message.contains("invalid district code format"));
    }

    #[test]
    fn login_form_fields_should_match_captured_payload_order_and_names() {
        let fields = build_jxemall_login_form_fields(&JxemallLoginFormInput {
            username: "HYJzhy18720378250".to_string(),
            password_value: "zcyFront::71b03527ea94dcaeede4ddf51edfa4a3".to_string(),
        })
        .expect("login form should be built");

        assert_eq!(
            fields,
            vec![
                ("platformCode".to_string(), "zcy".to_string()),
                ("loginType".to_string(), "password".to_string()),
                ("requestType".to_string(), "async".to_string()),
                ("username".to_string(), "HYJzhy18720378250".to_string()),
                (
                    "password".to_string(),
                    "zcyFront::71b03527ea94dcaeede4ddf51edfa4a3".to_string()
                ),
            ]
        );
        assert_eq!(
            JXEMALL_LOGIN_QUERY_CURRENT_URI,
            "current_uri=https%3A%2F%2Flogin.jxemall.com%2Fuser-login%2F%23%2F"
        );
    }

    #[test]
    fn login_form_fields_should_fail_when_username_is_empty() {
        let err = build_jxemall_login_form_fields(&JxemallLoginFormInput {
            username: "  ".to_string(),
            password_value: "zcyFront::xxx".to_string(),
        })
        .expect_err("empty username should fail");

        assert!(err.message.contains("username is required"));
    }

    #[test]
    fn login_form_fields_should_fail_when_password_value_is_empty() {
        let err = build_jxemall_login_form_fields(&JxemallLoginFormInput {
            username: "demo".to_string(),
            password_value: " ".to_string(),
        })
        .expect_err("empty password value should fail");

        assert!(err.message.contains("password value is required"));
    }

    #[test]
    fn persist_crawl_batch_should_upsert_and_mark_job_succeeded() {
        let db_file = NamedTempFile::new().expect("temp db should be created");
        let repo =
            SqliteStorageRepository::open(db_file.path()).expect("repo should be initialized");

        let records = vec![Record {
            source_id: "rid-1".to_string(),
            source_url: "https://example.local/1".to_string(),
            title: "弱电改造项目".to_string(),
            region_code: "360103".to_string(),
            published_at: 100,
            expires_at: 200,
        }];

        let upserted =
            persist_crawl_batch(&repo, "job-crawler-1", &records).expect("persist should pass");
        assert_eq!(upserted, 1);

        let status = repo
            .get_job_status("job-crawler-1")
            .expect("status query should pass");
        assert_eq!(status, Some(shared::CrawlJobStatus::Succeeded));

        let all = repo.list_records().expect("list should pass");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].source_id, "rid-1");
    }
}
