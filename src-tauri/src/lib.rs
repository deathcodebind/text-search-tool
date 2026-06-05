use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
#[cfg(not(test))]
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use crawler::persist_crawl_batch;
use gui::{CrawlerLoginApiClient, LoginApiClient, LoginRequest};
use once_cell::sync::Lazy;
use search_engine::SearchDocument;
use serde::{Deserialize, Serialize};
use shared::{Record, SourceSite};
#[cfg(not(test))]
use storage::RecordDetailPayload;
use storage::SqliteStorageRepository;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginInput {
  base_url: String,
  username: String,
  password: String,
  cookie_header: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
  credential_ref: String,
  expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequest {
  district_codes: Vec<String>,
  category_type: Option<String>,
  state_list: Vec<u32>,
  instance_codes: Vec<String>,
  min_budget: Option<u64>,
  max_budget: Option<u64>,
  sort_field: Option<String>,
  sort_method: Option<String>,
  keyword_hint: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullStartResponse {
  job_id: String,
  accepted: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PullProgressResponse {
  job_id: String,
  status: String,
  processed: u64,
  total: u64,
  message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullCancelResponse {
  job_id: String,
  accepted: bool,
  message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullRecordItem {
  source_id: String,
  source_url: String,
  title: String,
  region_code: String,
  published_at: i64,
  expires_at: i64,
  has_detail: bool,
  detail_status: String,
  detail_message: String,
  detail_attempts: u32,
  detail_updated_at: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullRecordsResponse {
  page: u32,
  page_size: u32,
  total: u64,
  records: Vec<PullRecordItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRecordsQuery {
  job_id: Option<String>,
  page: Option<u32>,
  page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullRecordDetailResponse {
  source_id: String,
  title: String,
  detail_text: String,
  raw_json: String,
  source_page_url: String,
  attachment_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullRecordStatusResponse {
  source_id: String,
  status: String,
  message: String,
  attempts: u32,
  updated_at: Option<i64>,
  has_detail: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchDetailPageHtmlResponse {
  source_page_url: String,
  html: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadAttachmentResponse {
  source_id: String,
  file_name: String,
  file_path: String,
  size: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenExternalUrlResponse {
  opened: bool,
  opened_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PullRetryDetailResponse {
  source_id: String,
  accepted: bool,
  message: String,
}

#[derive(Debug)]
struct PullFetchResult {
  records: Vec<Record>,
  detail_targets: Vec<PullDetailTarget>,
}

#[derive(Debug, Clone)]
struct PullDetailTarget {
  source_id: String,
  requisition_id: String,
  announcement_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum BoolNodeInput {
  Term { value: String },
  Bool {
    must: Vec<BoolNodeInput>,
    should: Vec<BoolNodeInput>,
    must_not: Vec<BoolNodeInput>,
    minimum_should_match: u32,
  },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoolPreviewRequest {
  query: BoolNodeInput,
  page: u32,
  page_size: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeywordGroupInput {
  occur: String,
  terms: Vec<String>,
  minimum_should_match: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeywordGroupsPreviewRequest {
  groups: Vec<KeywordGroupInput>,
  root_minimum_should_match: u32,
  page: u32,
  page_size: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
  app_name: String,
  version: String,
  capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewHit {
  source_id: String,
  title: String,
  region_code: String,
  snippet: String,
  score: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
  total: u64,
  hits: Vec<PreviewHit>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginSessionState {
  base_url: String,
  cookie_header: String,
}

static PULL_JOBS: Lazy<Mutex<HashMap<String, PullProgressResponse>>> =
  Lazy::new(|| Mutex::new(HashMap::new()));
static CANCELED_PULL_JOBS: Lazy<Mutex<HashSet<String>>> =
  Lazy::new(|| Mutex::new(HashSet::new()));
static LOGIN_SESSION_COOKIE: Lazy<Mutex<Option<String>>> =
  Lazy::new(|| Mutex::new(None));
static LOGIN_BASE_URL: Lazy<Mutex<Option<String>>> =
  Lazy::new(|| Mutex::new(None));

const DEFAULT_INSTANCE_CODES: [&str; 4] = ["JXWSCS", "JXDDCG", "JXXYGH", "JXFWGC"];
const DETAIL_JOB_STALLED_SECONDS: i64 = 120;
#[cfg(not(test))]
const PULL_FETCH_PAGE_SIZE: u32 = 100;

fn set_pull_job_progress(progress: PullProgressResponse) -> Result<(), String> {
  if progress.status != "canceled" {
    let canceled = CANCELED_PULL_JOBS
      .lock()
      .map_err(|_| "failed to lock canceled pull jobs state".to_string())?;
    if canceled.contains(&progress.job_id) {
      return Ok(());
    }
  }

  let mut jobs = PULL_JOBS
    .lock()
    .map_err(|_| "failed to lock pull jobs state".to_string())?;
  jobs.insert(progress.job_id.clone(), progress);
  Ok(())
}

fn is_pull_job_canceled(job_id: &str) -> Result<bool, String> {
  let canceled = CANCELED_PULL_JOBS
    .lock()
    .map_err(|_| "failed to lock canceled pull jobs state".to_string())?;
  Ok(canceled.contains(job_id))
}

fn execute_pull_job(
  job_id: String,
  input: PullRequest,
  session_cookie: String,
  effective_state_list: Vec<u32>,
  effective_sort_field: String,
  effective_sort_method: String,
  expires_at: i64,
) -> Result<(), String> {
  if is_pull_job_canceled(&job_id)? {
    return Ok(());
  }

  let pull_result = fetch_pull_records(
    &input,
    &session_cookie,
    &effective_state_list,
    &effective_sort_field,
    &effective_sort_method,
  )?;

  let mut persisted_records = pull_result.records;
  let has_detail_tasks = !pull_result.detail_targets.is_empty();

  if is_pull_job_canceled(&job_id)? {
    return Ok(());
  }

  for record in &mut persisted_records {
    record.expires_at = expires_at;
  }

  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let inserted = persist_crawl_batch(&repo, &job_id, &persisted_records).map_err(|e| e.message)?;

  #[cfg(not(test))]
  {
    if has_detail_tasks {
      for target in &pull_result.detail_targets {
        let _ = repo.upsert_record_detail_job_status(
          &target.source_id,
          "queued",
          Some("waiting for background detail sync"),
          1,
        );
      }

      let base_url = current_api_base_url()?;
      run_detail_sync_async(
        job_id.clone(),
        base_url,
        session_cookie.clone(),
        pull_result.detail_targets.clone(),
        1,
      );
    }
  }

  let summary = format!(
    "cookie={}, category={:?}, states={:?}, instances={:?}, budget={:?}-{:?}, sort={:?}/{:?}",
    session_cookie,
    input.category_type,
    effective_state_list,
    effective_instance_codes(&input),
    input.min_budget,
    input.max_budget,
    Some(effective_sort_field),
    Some(effective_sort_method),
  );

  set_pull_job_progress(PullProgressResponse {
    job_id,
    status: if has_detail_tasks {
      "running".to_string()
    } else {
      "succeeded".to_string()
    },
    processed: inserted,
    total: inserted,
    message: format!(
      "list pull completed, persisted {inserted} records; detail sync {} in background, {summary}",
      if has_detail_tasks { "running" } else { "skipped" }
    ),
  })?;

  Ok(())
}

fn extract_query_value(url: &str, key: &str) -> Option<String> {
  let query = url.split_once('?')?.1;
  for pair in query.split('&') {
    let mut kv = pair.splitn(2, '=');
    let k = kv.next()?;
    let v = kv.next().unwrap_or("");
    if k == key && !v.is_empty() {
      return Some(v.to_string());
    }
  }
  None
}

fn build_detail_target_from_source_url(source_id: String, source_url: &str) -> Option<PullDetailTarget> {
  let requisition_id = extract_query_value(source_url, "requisitionId")?;
  let announcement_type = extract_query_value(source_url, "type")
    .unwrap_or_else(|| "BIDDING_INVITATION".to_string());
  Some(PullDetailTarget {
    source_id,
    requisition_id,
    announcement_type,
  })
}

fn ensure_detail_type_query(raw_url: &str) -> String {
  if extract_query_value(raw_url, "type")
    .map(|value| !value.trim().is_empty())
    .unwrap_or(false)
  {
    return raw_url.to_string();
  }

  let (base, fragment) = match raw_url.split_once('#') {
    Some((left, right)) => (left, Some(right)),
    None => (raw_url, None),
  };

  let separator = if base.contains('?') { '&' } else { '?' };
  let mut merged = format!("{base}{separator}type=BIDDING_INVITATION");
  if let Some(fragment_value) = fragment {
    merged.push('#');
    merged.push_str(fragment_value);
  }
  merged
}

fn normalize_source_page_url(raw_url: &str) -> String {
  let trimmed = raw_url.trim();
  if trimmed.is_empty() {
    return String::new();
  }

  if !trimmed.contains("/api/sparta/announcement/detail") {
    if trimmed.contains("/luban/bidding/detail") {
      return ensure_detail_type_query(trimmed);
    }
    return trimmed.to_string();
  }

  let requisition_id = match extract_query_value(trimmed, "requisitionId") {
    Some(value) if !value.trim().is_empty() => value,
    _ => return trimmed.to_string(),
  };
  let announcement_type = extract_query_value(trimmed, "type")
    .unwrap_or_else(|| "BIDDING_INVITATION".to_string());

  crawler::JXEMALL_DETAIL_REFERER_TEMPLATE
    .replace("{requisitionId}", requisition_id.trim())
    .replace("{type}", announcement_type.trim())
}

fn extract_session_cookie_from_credential_ref(credential_ref: &str) -> Option<String> {
  const PREFIX: &str = "cred://jxemall/sso/";
  if let Some(sso_session) = credential_ref.strip_prefix(PREFIX) {
    let trimmed = sso_session.trim();
    if !trimmed.is_empty() {
      return Some(format!("SSOSESSION={trimmed}"));
    }
  }
  None
}

fn set_login_session_cookie_from_credential_ref(credential_ref: &str) -> Result<(), String> {
  let cookie = extract_session_cookie_from_credential_ref(credential_ref);
  let mut guard = LOGIN_SESSION_COOKIE
    .lock()
    .map_err(|_| "failed to lock login session state".to_string())?;
  *guard = cookie;
  Ok(())
}

fn require_login_session_cookie() -> Result<String, String> {
  {
    let guard = LOGIN_SESSION_COOKIE
      .lock()
      .map_err(|_| "failed to lock login session state".to_string())?;
    if let Some(cookie) = guard.as_ref() {
      return Ok(cookie.clone());
    }
  }

  if let Some(state) = load_login_session_state()? {
    {
      let mut cookie_guard = LOGIN_SESSION_COOKIE
        .lock()
        .map_err(|_| "failed to lock login session state".to_string())?;
      *cookie_guard = Some(state.cookie_header.clone());
    }
    {
      let mut base_guard = LOGIN_BASE_URL
        .lock()
        .map_err(|_| "failed to lock login base url state".to_string())?;
      *base_guard = Some(state.base_url);
    }
    return Ok(state.cookie_header);
  }

  Err("请先登录（缺少会话 cookie）".to_string())
}

fn set_login_base_url(base_url: &str) -> Result<(), String> {
  let mut guard = LOGIN_BASE_URL
    .lock()
    .map_err(|_| "failed to lock login base url state".to_string())?;
  *guard = Some(base_url.trim().to_string());
  Ok(())
}

fn canonical_login_base_url(raw: &str) -> String {
  let trimmed = raw.trim().trim_end_matches('/');
  if trimmed.contains("jxemall.com") {
    "https://login.jxemall.com".to_string()
  } else {
    trimmed.to_string()
  }
}

fn canonical_api_base_url(raw: &str) -> String {
  let trimmed = raw.trim().trim_end_matches('/');
  if trimmed.contains("jxemall.com") {
    "https://www.jxemall.com".to_string()
  } else {
    trimmed.to_string()
  }
}

fn current_api_base_url() -> Result<String, String> {
  let base = {
    let guard = LOGIN_BASE_URL
      .lock()
      .map_err(|_| "failed to lock login base url state".to_string())?;
    guard.as_ref().cloned()
  };

  let base = if let Some(base) = base {
    base
  } else if let Some(state) = load_login_session_state()? {
    let mut base_guard = LOGIN_BASE_URL
      .lock()
      .map_err(|_| "failed to lock login base url state".to_string())?;
    *base_guard = Some(state.base_url.clone());
    state.base_url
  } else {
    "https://www.jxemall.com".to_string()
  };

  if base.contains("jxemall.com") {
    return Ok("https://www.jxemall.com".to_string());
  }

  Ok(base)
}

fn preview_documents_from_storage() -> Result<Vec<SearchDocument>, String> {
  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let docs = repo.list_record_documents().map_err(|e| e.message)?;

  Ok(
    docs
      .into_iter()
      .map(|x| SearchDocument {
        source_id: x.record.source_id,
        title: x.record.title,
        content: format!(
          "{} {}",
          x.record.source_url,
          x.detail_text.unwrap_or_default()
        ),
        region_code: x.record.region_code,
      })
      .collect(),
  )
}

fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
  haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn evaluate_bool_query(node: &BoolNodeInput, text: &str) -> (bool, i64) {
  match node {
    BoolNodeInput::Term { value } => {
      if value.trim().is_empty() {
        return (false, 0);
      }
      let matched = contains_case_insensitive(text, value.trim());
      (matched, if matched { 10 } else { 0 })
    }
    BoolNodeInput::Bool {
      must,
      should,
      must_not,
      minimum_should_match,
    } => {
      let mut score = 0;

      for child in must {
        let (ok, child_score) = evaluate_bool_query(child, text);
        if !ok {
          return (false, 0);
        }
        score += child_score;
      }

      for child in must_not {
        let (ok, _) = evaluate_bool_query(child, text);
        if ok {
          return (false, 0);
        }
      }

      let mut should_hits = 0u32;
      for child in should {
        let (ok, child_score) = evaluate_bool_query(child, text);
        if ok {
          should_hits += 1;
          score += child_score;
        }
      }

      let required = if should.is_empty() { 0 } else { *minimum_should_match };
      if should_hits < required {
        return (false, 0);
      }

      (true, score)
    }
  }
}

fn evaluate_keyword_group(group: &KeywordGroupInput, text: &str) -> (bool, i64) {
  if group.terms.is_empty() {
    return (true, 0);
  }

  let mut hit_count = 0u32;
  for term in &group.terms {
    if contains_case_insensitive(text, term) {
      hit_count += 1;
    }
  }

  let required = if group.minimum_should_match == 0 {
    1
  } else {
    group.minimum_should_match.min(group.terms.len() as u32)
  };

  let passed = hit_count >= required;
  (passed, (hit_count as i64) * 10)
}

fn user_app_data_dir() -> Option<PathBuf> {
  #[cfg(target_os = "macos")]
  {
    if let Ok(home) = std::env::var("HOME") {
      let mut path = PathBuf::from(home);
      path.push("Library");
      path.push("Application Support");
      path.push("Text Search Tool");
      return Some(path);
    }
  }

  #[cfg(target_os = "windows")]
  {
    if let Ok(appdata) = std::env::var("APPDATA") {
      let mut path = PathBuf::from(appdata);
      path.push("Text Search Tool");
      return Some(path);
    }
  }

  #[cfg(not(any(target_os = "macos", target_os = "windows")))]
  {
    if let Ok(xdg_data_home) = std::env::var("XDG_DATA_HOME") {
      let mut path = PathBuf::from(xdg_data_home);
      path.push("text-search-tool");
      return Some(path);
    }

    if let Ok(home) = std::env::var("HOME") {
      let mut path = PathBuf::from(home);
      path.push(".local");
      path.push("share");
      path.push("text-search-tool");
      return Some(path);
    }
  }

  None
}

fn db_path() -> Result<PathBuf, String> {
  let mut candidate_dirs = Vec::new();

  if let Ok(mut cwd) = std::env::current_dir() {
    #[cfg(debug_assertions)]
    {
      // In dev, keep sqlite out of src-tauri/data to avoid file-watch rebuild loops.
      if cwd.ends_with("src-tauri") {
        cwd.pop();
      }
      let mut dev_dir = cwd.clone();
      dev_dir.push(".runtime-data");
      candidate_dirs.push(dev_dir);
    }

    #[cfg(not(debug_assertions))]
    {
      cwd.push("data");
      candidate_dirs.push(cwd);
    }
  }

  if let Some(mut user_dir) = user_app_data_dir() {
    user_dir.push("data");
    candidate_dirs.push(user_dir);
  }

  let mut errors = Vec::new();
  for mut dir in candidate_dirs {
    match std::fs::create_dir_all(&dir) {
      Ok(()) => {
        dir.push("app.db");
        return Ok(dir);
      }
      Err(e) => {
        errors.push(format!("{} ({e})", dir.display()));
      }
    }
  }

  Err(format!(
    "failed to create data dir in all candidates: {}",
    errors.join("; ")
  ))
}

fn session_state_path() -> Result<PathBuf, String> {
  let mut path = db_path()?;
  path.pop();
  path.push("session.json");
  Ok(path)
}

fn save_login_session_state(base_url: &str, cookie_header: &str) -> Result<(), String> {
  let path = session_state_path()?;
  let state = LoginSessionState {
    base_url: base_url.to_string(),
    cookie_header: cookie_header.to_string(),
  };
  let content = serde_json::to_vec(&state)
    .map_err(|e| format!("failed to serialize login session state: {e}"))?;
  std::fs::write(path, content).map_err(|e| format!("failed to persist login session state: {e}"))
}

fn read_login_session_state_from(path: &Path) -> Result<LoginSessionState, String> {
  let content =
    std::fs::read(path).map_err(|e| format!("failed to read login session state: {e}"))?;
  serde_json::from_slice::<LoginSessionState>(&content)
    .map_err(|e| format!("failed to parse login session state: {e}"))
}

fn legacy_session_state_paths() -> Vec<PathBuf> {
  let mut paths = Vec::new();

  if let Ok(mut cwd) = std::env::current_dir() {
    let mut p1 = cwd.clone();
    p1.push("data");
    p1.push("session.json");
    paths.push(p1);

    if !cwd.ends_with("src-tauri") {
      cwd.push("src-tauri");
    }
    cwd.push("data");
    cwd.push("session.json");
    paths.push(cwd);
  }

  paths
}

fn load_login_session_state() -> Result<Option<LoginSessionState>, String> {
  let path = session_state_path()?;
  if path.exists() {
    return read_login_session_state_from(&path).map(Some);
  }

  for legacy_path in legacy_session_state_paths() {
    if !legacy_path.exists() {
      continue;
    }
    let state = read_login_session_state_from(&legacy_path)?;
    let _ = save_login_session_state(&state.base_url, &state.cookie_header);
    return Ok(Some(state));
  }

  Ok(None)
}

fn clear_login_session_state() -> Result<(), String> {
  {
    let mut cookie_guard = LOGIN_SESSION_COOKIE
      .lock()
      .map_err(|_| "failed to lock login session state".to_string())?;
    *cookie_guard = None;
  }

  {
    let mut base_guard = LOGIN_BASE_URL
      .lock()
      .map_err(|_| "failed to lock login base url state".to_string())?;
    *base_guard = None;
  }

  let path = session_state_path()?;
  if path.exists() {
    std::fs::remove_file(&path)
      .map_err(|e| format!("failed to clear login session state: {e}"))?;
  }

  Ok(())
}

fn now_unix_secs() -> Result<i64, String> {
  Ok(
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| format!("failed to read clock: {e}"))?
      .as_secs() as i64,
  )
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct DemoPullRecord {
  record: Record,
  category_type: &'static str,
  state: u32,
  instance_code: &'static str,
  budget: u64,
}

#[cfg(test)]
fn demo_records_for_pull() -> Vec<DemoPullRecord> {
  vec![
    DemoPullRecord {
      record: Record {
        source_id: "pull-002".to_string(),
        source_url: "https://example.local/2".to_string(),
        title: "智慧校园安防升级".to_string(),
        region_code: "360111".to_string(),
        published_at: 1_715_000_100,
        expires_at: 1_817_000_100,
      },
      category_type: "SERVICE",
      state: 3,
      instance_code: "JXWSCS",
      budget: 54000,
    },
    DemoPullRecord {
      record: Record {
        source_id: "pull-003".to_string(),
        source_url: "https://example.local/3".to_string(),
        title: "家具采购项目".to_string(),
        region_code: "360104".to_string(),
        published_at: 1_715_000_200,
        expires_at: 1_817_000_200,
      },
      category_type: "GOODS",
      state: 5,
      instance_code: "JXDDCG",
      budget: 120000,
    },
  ]
}

fn effective_instance_codes(input: &PullRequest) -> Vec<String> {
  if input.instance_codes.is_empty() {
    return DEFAULT_INSTANCE_CODES
      .iter()
      .map(|x| (*x).to_string())
      .collect();
  }
  input.instance_codes.clone()
}

fn fetch_pull_records(
  input: &PullRequest,
  _session_cookie: &str,
  effective_state_list: &[u32],
  _effective_sort_field: &str,
  effective_sort_method: &str,
) -> Result<PullFetchResult, String> {
  #[cfg(test)]
  {
    let mut records = demo_records_for_pull();
    if let Some(category_type) = input.category_type.as_ref() {
      records.retain(|x| x.category_type.eq_ignore_ascii_case(category_type));
    }
    records.retain(|x| effective_state_list.iter().any(|s| *s == x.state));
    if !input.instance_codes.is_empty() {
      records.retain(|x| {
        input
          .instance_codes
          .iter()
          .any(|code| code.eq_ignore_ascii_case(x.instance_code))
      });
    }
    if let Some(min_budget) = input.min_budget {
      records.retain(|x| x.budget >= min_budget);
    }
    if let Some(max_budget) = input.max_budget {
      records.retain(|x| x.budget <= max_budget);
    }
    if !input.district_codes.is_empty() {
      records.retain(|x| input.district_codes.iter().any(|code| code == &x.record.region_code));
    }
    if let Some(keyword) = input.keyword_hint.as_ref() {
      let trimmed = keyword.trim();
      if !trimmed.is_empty() {
        records.retain(|x| contains_case_insensitive(&x.record.title, trimmed));
      }
    }

    let mut pulled: Vec<Record> = records.into_iter().map(|x| x.record).collect();
    if effective_sort_method == "ASC" {
      pulled.sort_by(|a, b| a.published_at.cmp(&b.published_at));
    } else {
      pulled.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    }
    return Ok(PullFetchResult {
      records: pulled,
      detail_targets: Vec::new(),
    });
  }

  #[cfg(not(test))]
  {
    let api_base = current_api_base_url()?;
    let client = crawler::JxemallListNewestHttpClient::new(api_base, _session_cookie.to_string())
      .map_err(|e| format!("failed to initialize listNewest client: {}", e.message))?;

    let payload = crawler::JxemallListNewestRequest {
      back_category_name: String::new(),
      trade_model: "BIDDING".to_string(),
      category_type: input.category_type.clone(),
      page_no: 1,
      page_size: PULL_FETCH_PAGE_SIZE,
      state_list: effective_state_list.to_vec(),
      other_search: input.keyword_hint.clone().unwrap_or_default(),
      min_budget: input.min_budget,
      max_budget: input.max_budget,
      instance_codes: effective_instance_codes(input),
      sort_field: _effective_sort_field.to_string(),
      sort_method: effective_sort_method.to_string(),
      district_code_list: input.district_codes.clone(),
      administrative_district_code_list: Vec::new(),
    };

    let summaries = client
      .list_newest_summaries(&payload)
      .map_err(|e| format!("listNewest request failed: {}", e.message))?;

    let mut pulled: Vec<Record> = summaries.iter().map(|x| x.record.clone()).collect();

    let detail_targets: Vec<PullDetailTarget> = summaries
      .iter()
      .map(|x| PullDetailTarget {
        source_id: x.record.source_id.clone(),
        requisition_id: x.requisition_id.clone(),
        announcement_type: x.announcement_type.clone(),
      })
      .collect();

    if effective_sort_method == "ASC" {
      pulled.sort_by(|a, b| a.published_at.cmp(&b.published_at));
    } else {
      pulled.sort_by(|a, b| b.published_at.cmp(&a.published_at));
    }
    Ok(PullFetchResult {
      records: pulled,
      detail_targets,
    })
  }
}

#[cfg(not(test))]
fn run_detail_sync_async(
  job_id: String,
  base_url: String,
  session_cookie: String,
  targets: Vec<PullDetailTarget>,
  attempt: u32,
) {
  thread::spawn(move || {
    let all_source_ids: Vec<String> = targets.iter().map(|x| x.source_id.clone()).collect();

    let mark_all_as_failed = |message: String| {
      if let Ok(repo) = SqliteStorageRepository::open(match db_path() {
        Ok(p) => p,
        Err(_) => return,
      }) {
        for source_id in &all_source_ids {
          let _ = repo.upsert_record_detail_job_status(source_id, "failed", Some(&message), attempt);
        }
      }
    };

    let client = match crawler::JxemallListNewestHttpClient::new(base_url, session_cookie) {
      Ok(c) => c,
      Err(e) => {
        mark_all_as_failed(format!("detail sync init failed: {}", e.message));
        if let Ok(mut jobs) = PULL_JOBS.lock() {
          jobs.insert(
            job_id.clone(),
            PullProgressResponse {
              job_id,
              status: "failed".to_string(),
              processed: 0,
              total: 0,
              message: format!("detail sync init failed: {}", e.message),
            },
          );
        }
        return;
      }
    };

    let total = targets.len() as u64;
    if let Ok(mut jobs) = PULL_JOBS.lock() {
      jobs.insert(
        job_id.clone(),
        PullProgressResponse {
          job_id: job_id.clone(),
          status: "running".to_string(),
          processed: 0,
          total,
          message: "list synced, detail sync running in background".to_string(),
        },
      );
    }

    let repo = match SqliteStorageRepository::open(match db_path() {
      Ok(p) => p,
      Err(e) => {
        mark_all_as_failed(format!("detail sync failed to open db: {e}"));
        if let Ok(mut jobs) = PULL_JOBS.lock() {
          jobs.insert(
            job_id.clone(),
            PullProgressResponse {
              job_id,
              status: "failed".to_string(),
              processed: 0,
              total,
              message: format!("detail sync failed to open db: {e}"),
            },
          );
        }
        return;
      }
    }) {
      Ok(r) => r,
      Err(e) => {
        mark_all_as_failed(format!("detail sync failed to open db: {}", e.message));
        if let Ok(mut jobs) = PULL_JOBS.lock() {
          jobs.insert(
            job_id.clone(),
            PullProgressResponse {
              job_id,
              status: "failed".to_string(),
              processed: 0,
              total,
              message: format!("detail sync failed to open db: {}", e.message),
            },
          );
        }
        return;
      }
    };

    let mut payloads = Vec::new();
    let mut processed = 0u64;

    for target in targets {
      let _ = repo.upsert_record_detail_job_status(
        &target.source_id,
        "running",
        Some("detail sync running"),
        attempt,
      );

      match client.fetch_detail_data(&target.requisition_id, &target.announcement_type) {
        Ok(detail) => {
          payloads.push(RecordDetailPayload {
            source_id: target.source_id.clone(),
            detail_text: detail.detail_text,
            raw_json: detail.raw_json,
            source_page_url: detail.source_page_url,
            attachment_urls: detail.attachment_urls,
          });

          let _ = repo.upsert_record_detail_job_status(
            &target.source_id,
            "succeeded",
            Some("detail sync completed"),
            attempt,
          );
        }
        Err(e) => {
          let msg = e.message.to_lowercase();
          let status = if msg.contains("timed out") || msg.contains("timeout") {
            "timeout"
          } else {
            "failed"
          };

          let _ = repo.upsert_record_detail_job_status(
            &target.source_id,
            status,
            Some(&e.message),
            attempt,
          );
        }
      }

      processed += 1;
      if let Ok(mut jobs) = PULL_JOBS.lock() {
        jobs.insert(
          job_id.clone(),
          PullProgressResponse {
            job_id: job_id.clone(),
            status: "running".to_string(),
            processed,
            total,
            message: "detail sync running in background".to_string(),
          },
        );
      }
    }

    let saved = repo
      .upsert_record_detail_payloads(&payloads)
      .map(|x| x.to_string())
      .unwrap_or_else(|e| format!("failed to save detail payloads: {}", e.message));

    if let Ok(mut jobs) = PULL_JOBS.lock() {
      jobs.insert(
        job_id.clone(),
        PullProgressResponse {
          job_id,
          status: "succeeded".to_string(),
          processed,
          total,
          message: format!("detail sync completed, saved {saved}"),
        },
      );
    }
  });
}

#[tauri::command]
fn app_snapshot() -> AppSnapshot {
  AppSnapshot {
    app_name: "Text Search Tool".to_string(),
    version: env!("CARGO_PKG_VERSION").to_string(),
    capabilities: vec![
      "login-flow".to_string(),
      "pull-flow".to_string(),
      "nested-bool-query".to_string(),
    ],
  }
}

#[tauri::command]
fn login_submit(input: LoginInput) -> Result<LoginResponse, String> {
  let login_base_url = canonical_login_base_url(&input.base_url);
  let api_base_url = canonical_api_base_url(&input.base_url);

  let client = CrawlerLoginApiClient::new(login_base_url, input.cookie_header)
    .map_err(|e| format!("failed to initialize login client: {}", e.message))?;

  let response = client
    .login(&LoginRequest {
      source: SourceSite::Jxemall,
      username: input.username,
      password: input.password,
    })
    .map_err(|e| e.message)?;

  set_login_session_cookie_from_credential_ref(&response.credential_ref)?;
  set_login_base_url(&api_base_url)?;

  if let Some(cookie_header) = require_login_session_cookie().ok() {
    let _ = save_login_session_state(&api_base_url, &cookie_header);
  }

  Ok(LoginResponse {
    credential_ref: response.credential_ref,
    expires_at: response.expires_at,
  })
}

#[tauri::command]
fn pull_start(input: PullRequest) -> Result<PullStartResponse, String> {
  let session_cookie = require_login_session_cookie()?;

  let effective_state_list = if input.state_list.is_empty() {
    vec![4]
  } else {
    input.state_list.clone()
  };

  let effective_sort_field = input
    .sort_field
    .clone()
    .unwrap_or_else(|| "ANNOUNCEMENT_PUBLISH_TIME".to_string());
  let effective_sort_method = input
    .sort_method
    .clone()
    .unwrap_or_else(|| "DESC".to_string());

  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map_err(|e| format!("failed to read clock: {e}"))?
    .as_millis();
  let now_secs = (now / 1000) as i64;
  let expires_at = now_secs + 3 * 24 * 60 * 60;
  let job_id = format!("job-{now}");

  set_pull_job_progress(PullProgressResponse {
    job_id: job_id.clone(),
    status: "running".to_string(),
    processed: 0,
    total: 0,
    message: "pull started with filters".to_string(),
  })?;

  #[cfg(test)]
  {
    execute_pull_job(
      job_id.clone(),
      input,
      session_cookie,
      effective_state_list,
      effective_sort_field,
      effective_sort_method,
      expires_at,
    )?;
  }

  #[cfg(not(test))]
  {
    let job_id_for_thread = job_id.clone();
    let input_for_thread = input;
    let session_cookie_for_thread = session_cookie;
    let effective_state_list_for_thread = effective_state_list;
    let effective_sort_field_for_thread = effective_sort_field;
    let effective_sort_method_for_thread = effective_sort_method;

    thread::spawn(move || {
      if let Err(error) = execute_pull_job(
        job_id_for_thread.clone(),
        input_for_thread,
        session_cookie_for_thread,
        effective_state_list_for_thread,
        effective_sort_field_for_thread,
        effective_sort_method_for_thread,
        expires_at,
      ) {
        let _ = set_pull_job_progress(PullProgressResponse {
          job_id: job_id_for_thread,
          status: "failed".to_string(),
          processed: 0,
          total: 0,
          message: error,
        });
      }
    });
  }

  Ok(PullStartResponse {
    job_id,
    accepted: true,
  })
}

#[tauri::command]
fn pull_progress(job_id: String) -> Result<PullProgressResponse, String> {
  {
    let jobs = PULL_JOBS
      .lock()
      .map_err(|_| "failed to lock pull jobs state".to_string())?;

    if let Some(progress) = jobs.get(&job_id).cloned() {
      return Ok(progress);
    }
  }

  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let status = repo
    .get_job_status(&job_id)
    .map_err(|e| e.message)?
    .ok_or_else(|| "job not found".to_string())?;
  let processed = repo.count_job_records(&job_id).map_err(|e| e.message)?;
  let message = repo
    .get_job_message(&job_id)
    .map_err(|e| e.message)?
    .unwrap_or_else(|| "progress restored from storage".to_string());

  let progress = PullProgressResponse {
    job_id: job_id.clone(),
    status: if message.to_ascii_lowercase().contains("canceled") {
      "canceled".to_string()
    } else {
      match status {
      shared::CrawlJobStatus::Pending => "pending".to_string(),
      shared::CrawlJobStatus::Running => "running".to_string(),
      shared::CrawlJobStatus::Succeeded => "succeeded".to_string(),
      shared::CrawlJobStatus::Failed => "failed".to_string(),
      }
    },
    processed,
    total: processed,
    message,
  };

  let _ = set_pull_job_progress(progress.clone());
  Ok(progress)
}

#[tauri::command]
fn pull_cancel(job_id: String) -> Result<PullCancelResponse, String> {
  {
    let mut canceled = CANCELED_PULL_JOBS
      .lock()
      .map_err(|_| "failed to lock canceled pull jobs state".to_string())?;
    canceled.insert(job_id.clone());
  }

  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let _ = storage::StorageRepository::mark_job_status(
    &repo,
    &job_id,
    shared::CrawlJobStatus::Failed,
    Some("canceled by user"),
  );

  let processed = repo.count_job_records(&job_id).unwrap_or(0);
  let progress = PullProgressResponse {
    job_id: job_id.clone(),
    status: "canceled".to_string(),
    processed,
    total: processed,
    message: "当前任务已取消".to_string(),
  };
  let _ = set_pull_job_progress(progress);

  Ok(PullCancelResponse {
    job_id,
    accepted: true,
    message: "当前任务已取消".to_string(),
  })
}

#[tauri::command]
fn pull_records(input: Option<PullRecordsQuery>) -> Result<PullRecordsResponse, String> {
  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let job_id = input
    .as_ref()
    .and_then(|x| x.job_id.as_ref())
    .map(|x| x.trim())
    .filter(|x| !x.is_empty())
    .map(|x| x.to_string());
  let all = if let Some(job_id) = job_id.as_deref() {
    repo
      .list_record_documents_by_job(job_id)
      .map_err(|e| e.message)?
  } else {
    Vec::new()
  };
  let now_secs = now_unix_secs()?;
  let total = all.len() as u64;
  let page = input
    .as_ref()
    .and_then(|x| x.page)
    .unwrap_or(1)
    .max(1);
  let page_size = input
    .as_ref()
    .and_then(|x| x.page_size)
    .unwrap_or(20)
    .clamp(1, 200);
  let offset = ((page - 1) * page_size) as usize;

  let records = all
    .into_iter()
    .skip(offset)
    .take(page_size as usize)
    .map(|x| {
      let mut detail_status = x.detail_status.unwrap_or_else(|| "queued".to_string());
      let mut detail_message = x.detail_message.unwrap_or_default();

      // queued/running 长时间无更新时间时，视为卡住并允许重试。
      if (detail_status == "queued" || detail_status == "running")
        && x
          .detail_updated_at
          .map(|updated| now_secs.saturating_sub(updated) > DETAIL_JOB_STALLED_SECONDS)
          .unwrap_or(false)
      {
        let stale_from = detail_status.clone();
        detail_status = "timeout".to_string();
        detail_message = if detail_message.trim().is_empty() {
          format!(
            "detail sync stalled in {stale_from} for over {DETAIL_JOB_STALLED_SECONDS}s, please retry"
          )
        } else {
          format!(
            "detail sync stalled in {stale_from} for over {DETAIL_JOB_STALLED_SECONDS}s, please retry; last message: {}",
            detail_message
          )
        };
      }

      PullRecordItem {
        source_id: x.record.source_id,
        source_url: normalize_source_page_url(&x.record.source_url),
        title: x.record.title,
        region_code: x.record.region_code,
        published_at: x.record.published_at,
        expires_at: x.record.expires_at,
        has_detail: x.detail_text.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false),
        detail_status,
        detail_message,
        // 对外暴露“重试次数”，不计首次自动尝试。
        detail_attempts: x.detail_attempts.saturating_sub(1),
        detail_updated_at: x.detail_updated_at,
      }
    })
    .collect();

  Ok(PullRecordsResponse {
    page,
    page_size,
    total,
    records,
  })
}

#[tauri::command]
fn pull_record_detail(source_id: String) -> Result<PullRecordDetailResponse, String> {
  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let title = repo
    .get_record_by_source_id(&source_id)
    .map_err(|e| e.message)?
    .map(|x| x.title)
    .unwrap_or_default();
  let payload = repo
    .get_record_detail_payload(&source_id)
    .map_err(|e| e.message)?
    .ok_or_else(|| "detail not found, please wait for background sync".to_string())?;

  Ok(PullRecordDetailResponse {
    source_id: payload.source_id,
    title,
    detail_text: payload.detail_text,
    raw_json: payload.raw_json,
    source_page_url: normalize_source_page_url(&payload.source_page_url),
    attachment_urls: payload.attachment_urls,
  })
}

#[tauri::command]
fn pull_record_status(source_id: String) -> Result<PullRecordStatusResponse, String> {
  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let docs = repo.list_record_documents().map_err(|e| e.message)?;
  let doc = docs
    .into_iter()
    .find(|x| x.record.source_id == source_id)
    .ok_or_else(|| "record not found".to_string())?;

  let now_secs = now_unix_secs()?;
  let mut status = doc.detail_status.unwrap_or_else(|| "queued".to_string());
  let mut message = doc.detail_message.unwrap_or_default();

  if (status == "queued" || status == "running")
    && doc
      .detail_updated_at
      .map(|updated| now_secs.saturating_sub(updated) > DETAIL_JOB_STALLED_SECONDS)
      .unwrap_or(false)
  {
    let stale_from = status.clone();
    status = "timeout".to_string();
    message = if message.trim().is_empty() {
      format!(
        "detail sync stalled in {stale_from} for over {DETAIL_JOB_STALLED_SECONDS}s, please retry"
      )
    } else {
      format!(
        "detail sync stalled in {stale_from} for over {DETAIL_JOB_STALLED_SECONDS}s, please retry; last message: {}",
        message
      )
    };
  }

  Ok(PullRecordStatusResponse {
    source_id: doc.record.source_id,
    status,
    message,
    attempts: doc.detail_attempts.saturating_sub(1),
    updated_at: doc.detail_updated_at,
    has_detail: doc
      .detail_text
      .as_ref()
      .map(|t| !t.trim().is_empty())
      .unwrap_or(false),
  })
}

#[tauri::command]
fn fetch_detail_page_html(source_id: String) -> Result<FetchDetailPageHtmlResponse, String> {
  let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;
  let detail_payload = repo
    .get_record_detail_payload(&source_id)
    .map_err(|e| e.message)?;
  let source_page_url = if let Some(payload) = detail_payload {
    normalize_source_page_url(&payload.source_page_url)
  } else {
    let raw_url = repo
      .get_record_by_source_id(&source_id)
      .map_err(|e| e.message)?
      .map(|x| x.source_url)
      .ok_or_else(|| "record not found".to_string())?;
    normalize_source_page_url(&raw_url)
  };
  if source_page_url.trim().is_empty() {
    return Err("source page url is not available".to_string());
  }

  let session_cookie = require_login_session_cookie()?;
  let api_base = current_api_base_url()?;
  let client = crawler::JxemallListNewestHttpClient::new(api_base, session_cookie)
    .map_err(|e| e.message)?;
  let html = client.fetch_page_html(&source_page_url).map_err(|e| e.message)?;

  Ok(FetchDetailPageHtmlResponse {
    source_page_url,
    html,
  })
}

fn decode_url_file_name(url: &str) -> Option<String> {
  let path = url.split('?').next().unwrap_or(url);
  let name = path.rsplit('/').next()?.trim();
  if name.is_empty() {
    return None;
  }
  Some(name.replace('%', "_"))
}

fn safe_file_name(name: &str) -> String {
  let mut s = String::new();
  for ch in name.chars() {
    if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
      s.push(ch);
    } else {
      s.push('_');
    }
  }
  let trimmed = s.trim_matches('_');
  if trimmed.is_empty() {
    "attachment.bin".to_string()
  } else {
    trimmed.to_string()
  }
}

#[tauri::command]
fn download_attachment(source_id: String, url: String) -> Result<DownloadAttachmentResponse, String> {
  let trimmed = url.trim();
  if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
    return Err("invalid attachment url".to_string());
  }

  let session_cookie = require_login_session_cookie()?;
  let api_base = current_api_base_url()?;
  let client = crawler::JxemallListNewestHttpClient::new(api_base, session_cookie)
    .map_err(|e| e.message)?;
  let data = client
    .download_attachment_bytes(trimmed)
    .map_err(|e| e.message)?;

  let mut dir = db_path()?;
  let _ = dir.pop();
  dir.push("attachments");
  std::fs::create_dir_all(&dir)
    .map_err(|e| format!("failed to create attachments dir: {e}"))?;

  let guessed = decode_url_file_name(trimmed).unwrap_or_else(|| format!("{source_id}.bin"));
  let file_name = safe_file_name(&guessed);
  let unique_name = format!(
    "{}-{}-{}",
    source_id,
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map_err(|e| format!("failed to read clock: {e}"))?
      .as_millis(),
    file_name
  );

  let mut path = dir;
  path.push(unique_name);
  std::fs::write(&path, &data)
    .map_err(|e| format!("failed to write attachment file: {e}"))?;

  Ok(DownloadAttachmentResponse {
    source_id,
    file_name,
    file_path: path.to_string_lossy().to_string(),
    size: data.len() as u64,
  })
}

#[tauri::command]
fn open_external_url(url: String) -> Result<OpenExternalUrlResponse, String> {
  let trimmed = url.trim();
  if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
    return Err("invalid url".to_string());
  }

  #[cfg(target_os = "macos")]
  let mut cmd = {
    let mut c = Command::new("open");
    c.arg(trimmed);
    c
  };

  #[cfg(target_os = "linux")]
  let mut cmd = {
    let mut c = Command::new("xdg-open");
    c.arg(trimmed);
    c
  };

  #[cfg(target_os = "windows")]
  let mut cmd = {
    let mut c = Command::new("explorer");
    c.arg(trimmed);
    c
  };

  cmd
    .spawn()
    .map_err(|e| format!("failed to open external url: {e}"))?;

  Ok(OpenExternalUrlResponse {
    opened: true,
    opened_url: trimmed.to_string(),
  })
}

#[tauri::command]
fn clear_login_session() -> Result<(), String> {
  clear_login_session_state()
}

#[tauri::command]
fn pull_retry_detail(source_id: String) -> Result<PullRetryDetailResponse, String> {
  #[cfg(test)]
  {
    return Ok(PullRetryDetailResponse {
      source_id,
      accepted: true,
      message: "retry skipped in test build".to_string(),
    });
  }

  #[cfg(not(test))]
  {
    let session_cookie = require_login_session_cookie()?;
    let base_url = current_api_base_url()?;
    let repo = SqliteStorageRepository::open(db_path()?).map_err(|e| e.message)?;

    let record = repo
      .get_record_by_source_id(&source_id)
      .map_err(|e| e.message)?
      .ok_or_else(|| "record not found".to_string())?;

    let target = build_detail_target_from_source_url(source_id.clone(), &record.source_url)
      .ok_or_else(|| "cannot parse requisitionId/type from source url".to_string())?;

    let all_docs = repo.list_record_documents().map_err(|e| e.message)?;
    let attempts = all_docs
      .iter()
      .find(|x| x.record.source_id == source_id)
      .map(|x| x.detail_attempts + 1)
      .unwrap_or(1);

    repo
      .upsert_record_detail_job_status(
        &source_id,
        "queued",
        Some("queued for manual retry"),
        attempts,
      )
      .map_err(|e| e.message)?;

    let retry_job_id = format!(
      "retry-{}",
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("failed to read clock: {e}"))?
        .as_millis()
    );
    run_detail_sync_async(retry_job_id, base_url, session_cookie, vec![target], attempts);

    Ok(PullRetryDetailResponse {
      source_id,
      accepted: true,
      message: "detail retry started in background".to_string(),
    })
  }
}

#[tauri::command]
fn preview_bool_query(input: BoolPreviewRequest) -> Result<PreviewResponse, String> {
  let page = input.page.max(1);
  let page_size = input.page_size.max(1).min(100);

  let mut matched = Vec::new();
  for doc in preview_documents_from_storage()? {
    let text = format!("{} {}", doc.title, doc.content);
    let (ok, score) = evaluate_bool_query(&input.query, &text);
    if ok {
      matched.push(PreviewHit {
        source_id: doc.source_id,
        title: doc.title,
        region_code: doc.region_code,
        snippet: doc.content,
        score,
      });
    }
  }

  matched.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.source_id.cmp(&b.source_id)));

  let total = matched.len() as u64;
  let offset = ((page - 1) * page_size) as usize;
  let paged = matched
    .into_iter()
    .skip(offset)
    .take(page_size as usize)
    .collect();

  Ok(PreviewResponse {
    total,
    hits: paged,
  })
}

#[tauri::command]
fn preview_keyword_groups(input: KeywordGroupsPreviewRequest) -> Result<PreviewResponse, String> {
  let page = input.page.max(1);
  let page_size = input.page_size.max(1).min(100);

  let mut must_groups = Vec::new();
  let mut should_groups = Vec::new();
  let mut must_not_groups = Vec::new();

  for group in input.groups {
    match group.occur.as_str() {
      "must" => must_groups.push(group),
      "mustNot" => must_not_groups.push(group),
      _ => should_groups.push(group),
    }
  }

  let mut hits = Vec::new();
  for doc in preview_documents_from_storage()? {
    let text = format!("{} {}", doc.title, doc.content);
    let mut score = 0i64;

    let mut blocked = false;
    for group in &must_not_groups {
      let (ok, _) = evaluate_keyword_group(group, &text);
      if ok {
        blocked = true;
        break;
      }
    }
    if blocked {
      continue;
    }

    let mut all_must_passed = true;
    for group in &must_groups {
      let (ok, s) = evaluate_keyword_group(group, &text);
      if !ok {
        all_must_passed = false;
        break;
      }
      score += s;
    }
    if !all_must_passed {
      continue;
    }

    let mut should_hits = 0u32;
    for group in &should_groups {
      let (ok, s) = evaluate_keyword_group(group, &text);
      if ok {
        should_hits += 1;
        score += s;
      }
    }

    if should_groups.is_empty() {
      hits.push(PreviewHit {
        source_id: doc.source_id,
        title: doc.title,
        region_code: doc.region_code,
        snippet: doc.content,
        score,
      });
      continue;
    }

    if should_hits >= input.root_minimum_should_match {
      hits.push(PreviewHit {
        source_id: doc.source_id,
        title: doc.title,
        region_code: doc.region_code,
        snippet: doc.content,
        score,
      });
    }
  }

  hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.source_id.cmp(&b.source_id)));

  let total = hits.len() as u64;
  let offset = ((page - 1) * page_size) as usize;
  let paged = hits.into_iter().skip(offset).take(page_size as usize).collect();

  Ok(PreviewResponse { total, hits: paged })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      app_snapshot,
      login_submit,
      pull_start,
      pull_cancel,
      pull_progress,
      pull_records,
      pull_record_status,
      pull_record_detail,
      fetch_detail_page_html,
      download_attachment,
      open_external_url,
      clear_login_session,
      pull_retry_detail,
      preview_bool_query,
      preview_keyword_groups
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use once_cell::sync::Lazy;

  use std::sync::Mutex;

  use super::{
    PullRecordsQuery, PullRequest, extract_session_cookie_from_credential_ref, pull_records,
    pull_start,
    set_login_session_cookie_from_credential_ref,
  };

  static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

  #[test]
  fn should_extract_sso_cookie_from_credential_ref() {
    let _guard = TEST_MUTEX.lock().expect("test lock should work");
    let cookie = extract_session_cookie_from_credential_ref("cred://jxemall/sso/abc123");
    assert_eq!(cookie.as_deref(), Some("SSOSESSION=abc123"));
  }

  #[test]
  fn pull_should_fail_without_login_session_cookie() {
    let _guard = TEST_MUTEX.lock().expect("test lock should work");
    set_login_session_cookie_from_credential_ref("cred://jxemall/user/123")
      .expect("session setter should work");

    let result = pull_start(PullRequest {
      district_codes: vec!["360103".to_string()],
      category_type: None,
      state_list: vec![3, 4],
      instance_codes: vec!["JXWSCS".to_string()],
      min_budget: None,
      max_budget: None,
      sort_field: Some("ANNOUNCEMENT_PUBLISH_TIME".to_string()),
      sort_method: Some("DESC".to_string()),
      keyword_hint: None,
    });

    let err = result.expect_err("pull should be blocked before login");
    assert!(err.contains("请先登录"));
  }

  #[test]
  fn pull_should_default_to_in_progress_and_expire_in_3_days() {
    let _guard = TEST_MUTEX.lock().expect("test lock should work");
    set_login_session_cookie_from_credential_ref("cred://jxemall/sso/session-001")
      .expect("session setter should work");

    let now = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock should work")
      .as_secs() as i64;

    let started = pull_start(PullRequest {
      district_codes: vec!["360103".to_string(), "360111".to_string()],
      category_type: None,
      state_list: vec![3],
      instance_codes: Vec::new(),
      min_budget: None,
      max_budget: None,
      sort_field: None,
      sort_method: None,
      keyword_hint: None,
    })
    .expect("pull should pass after login");

    let result = pull_records(Some(PullRecordsQuery {
      job_id: Some(started.job_id),
      page: Some(1),
      page_size: Some(20),
    }))
    .expect("pull records should be listed");
    assert!(result.total >= 1);

    let first = result
      .records
      .first()
      .expect("at least one pulled record should exist");
    let expected_expires_at = now + 3 * 24 * 60 * 60;
    assert!(first.expires_at >= expected_expires_at - 5);
    assert!(first.expires_at <= expected_expires_at + 5);
  }
}
