# 全文搜索竞标助手 Rust 类型定义草案（MVP）

## 1. 目标

本文件提供可直接落地到 crates/shared 与各业务 crate 的 Rust 类型定义草案，优先保证：
- 模块边界清晰
- 错误语义统一
- 前后端字段一致

## 2. 建议目录

- crates/shared/src/model/
- crates/shared/src/error/
- crates/shared/src/paging/
- crates/shared/src/command/

## 3. 公共基础类型

```rust
use serde::{Deserialize, Serialize};

pub type UtcTime = String; // ISO 8601 UTC string
pub type RecordId = String;
pub type RuleId = String;
pub type JobId = String;
pub type SourceRecordId = String;
```

## 4. 分页与排序

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paging {
    pub page: u32,      // starts from 1
    pub page_size: u32, // suggested default: 20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    Score,
    PublishedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sort {
    pub sort_by: SortBy,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page<T> {
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub items: Vec<T>,
}
```

## 5. 核心领域模型

### 5.1 Source

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceSite {
    Jxemall,
}
```

### 5.2 Record

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordExtra {
    pub requisition_id: Option<String>,
    pub start_timestamp: Option<i64>,
    pub remaining_milliseconds: Option<i64>,
    pub state: Option<i32>,
    pub district_code: Option<String>,
    pub org_name: Option<String>,
    pub budget: Option<i64>,
    pub deal_amount: Option<i64>,
    pub item_type: Option<String>,
    pub category_type: Option<i32>,
    pub category_type_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub record_id: RecordId,
    pub source: SourceSite,
    pub source_record_id: SourceRecordId,
    pub source_url: String,
    pub title: String,
    pub content: String,
    pub region: Option<String>,
    pub published_at: Option<UtcTime>,
    pub deadline_at: Option<UtcTime>,
    pub extra: RecordExtra,
    pub created_at: UtcTime,
    pub updated_at: UtcTime,
    pub expired: bool,
}
```

说明：
- `sub_state` 保留在原始响应结构 `JxemallListItemRaw` 中，用于调试和后续研究。
- MVP 持久化模型 `RecordExtra` 暂不承载 `sub_state`。
- 返回值中的 `trade_model` / `trade_style` / `display_trade_style` 保留在 `JxemallListItemRaw`，不进入 `RecordExtra`。

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JxemallListItemRaw {
    pub announcement_id: Option<i64>,
    pub bidding_id: i64,
    pub requisition_id: Option<String>,
    pub title: String,
    pub pub_timestamp: Option<i64>,
    pub start_timestamp: Option<i64>,
    pub end_timestamp: Option<i64>,
    pub remaining_milliseconds: Option<i64>,
    pub state: Option<i32>,
    pub sub_state: Option<i32>,
    pub district_name: Option<String>,
    pub district_code: Option<String>,
    pub org_name: Option<String>,
    pub budget: Option<i64>,
    pub deal_amount: Option<i64>,
    pub r#type: Option<String>,
    pub province_name: Option<String>,
    pub city_name: Option<String>,
    pub area_name: Option<String>,
    pub trade_model: Option<String>,
    pub trade_style: Option<String>,
    pub category_type: Option<i32>,
    pub category_type_text: Option<String>,
    pub display_trade_style: Option<String>,
}
```

### 5.3 Rule Query

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryTerm {
    pub term: String,
    pub boost: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleQuery {
    pub must: Vec<QueryTerm>,
    pub should: Vec<QueryTerm>,
    pub must_not: Vec<QueryTerm>,
    pub minimum_should_match: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreWeights {
    pub a_keyword: f32,
    pub b_field: f32,
    pub c_freshness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: RuleId,
    pub name: String,
    pub enabled: bool,
    pub query: RuleQuery,
    pub score_weights: ScoreWeights,
    pub created_at: UtcTime,
    pub updated_at: UtcTime,
}
```

### 5.4 CrawlJob

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CrawlProgress {
    pub fetched: u64,
    pub upserted: u64,
    pub deduplicated: u64,
    pub indexed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlWindow {
    pub start: UtcTime,
    pub end: UtcTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlJob {
    pub job_id: JobId,
    pub source: SourceSite,
    pub status: CrawlJobStatus,
    pub window: CrawlWindow,
    pub progress: CrawlProgress,
    pub error: Option<AppError>,
    pub started_at: Option<UtcTime>,
    pub finished_at: Option<UtcTime>,
}
```

## 6. 搜索结果类型

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub keyword: f32,
    pub field: f32,
    pub freshness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchItem {
    pub record_id: RecordId,
    pub title: String,
    pub region: Option<String>,
    pub published_at: Option<UtcTime>,
    pub source_url: String,
    pub highlights: Vec<String>,
    pub score: f32,
    pub score_breakdown: Option<ScoreBreakdown>,
}

pub type SearchPage = Page<SearchItem>;
```

## 7. Command 入参与出参

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveCredentialReq {
    pub source: SourceSite,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveCredentialResp {
    pub credential_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateSessionReq {
    pub source: SourceSite,
    pub credential_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateSessionResp {
    pub valid: bool,
    pub expires_at: Option<UtcTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartCrawlReq {
    pub source: SourceSite,
    pub credential_ref: String,
    pub filters: Value,
    pub window: Option<CrawlWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartCrawlResp {
    pub job_id: JobId,
    pub status: CrawlJobStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCrawlJobReq {
    pub job_id: JobId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRecordsReq {
    pub rule_id: RuleId,
    pub paging: Paging,
    pub sort: Sort,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRecordDetailReq {
    pub record_id: RecordId,
}
```

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JxemallCategoryType {
    #[serde(rename = "GOODS")]
    Goods,
    #[serde(rename = "SERVICE")]
    Service,
    // 语义取自中文筛选项“工程类”，仅因对端接口约束序列化为 PROJECT。
    #[serde(rename = "PROJECT")]
    Engineering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JxemallTradeModel {
    #[serde(rename = "BIDDING")]
    Bidding,
    #[serde(rename = "REVERSE")]
    Reverse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JxemallInstanceCode {
    #[serde(rename = "JXWSCS")]
    OnlineSupermarket,
    #[serde(rename = "JXDDCG")]
    FixedPointProcurementHall,
    #[serde(rename = "JXXYGH")]
    AgreementSupplyHall,
    #[serde(rename = "JXFWGC")]
    ServiceEngineeringHall,
}

impl JxemallInstanceCode {
    pub fn all_in_ui_order() -> Vec<Self> {
        vec![
            Self::OnlineSupermarket,
            Self::FixedPointProcurementHall,
            Self::AgreementSupplyHall,
            Self::ServiceEngineeringHall,
        ]
    }
}

#[derive(Debug, Clone)]
pub enum JxemallBiddingStateFilter {
    All,
    NotStarted,
    InProgress,
    Expired,
}

impl JxemallBiddingStateFilter {
    pub fn to_state_list(&self) -> Vec<i32> {
        match self {
            Self::All => vec![3, 4, 5, 6, 7, 10, 12, 50],
            Self::NotStarted => vec![3],
            Self::InProgress => vec![4],
            Self::Expired => vec![5, 6, 7, 10, 12, 50],
        }
    }
}

impl JxemallBiddingStateFilter {
    pub fn all_codes() -> Vec<i32> {
        Self::All.to_state_list()
    }
}

#[derive(Debug, Clone)]
pub enum JxemallBudgetFilter {
    All,
    LessThan50k,
    Between50kAnd60k,
    Between60kAnd70k,
    AtLeast100k,
}

impl JxemallBudgetFilter {
    pub fn to_budget_range(&self) -> (Option<i64>, Option<i64>) {
        match self {
            Self::All => (None, None),
            Self::LessThan50k => (None, Some(50_000)),
            Self::Between50kAnd60k => (Some(50_000), Some(60_000)),
            Self::Between60kAnd70k => (Some(60_000), Some(70_000)),
            Self::AtLeast100k => (Some(100_000), None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JxemallSortField {
    #[serde(rename = "ANNOUNCEMENT_PUBLISH_TIME")]
    AnnouncementPublishTime,
    #[serde(rename = "QUOTE_DEADLINE")]
    QuoteDeadline,
    #[serde(rename = "BUDGET_AMOUNT")]
    BudgetAmount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JxemallSortMethod {
    #[serde(rename = "ASC")]
    Asc,
    #[serde(rename = "DESC")]
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JxemallListNewestPayload {
    pub back_category_name: String,
    pub trade_model: JxemallTradeModel,
    pub category_type: Option<JxemallCategoryType>,
    pub page_no: u32,
    pub page_size: u32,
    pub state_list: Vec<i32>,
    pub other_search: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_budget: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_budget: Option<i64>,
    pub instance_codes: Vec<JxemallInstanceCode>,
    pub sort_field: JxemallSortField,
    pub sort_method: JxemallSortMethod,
    pub district_code_list: Vec<String>,
    pub administrative_district_code_list: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum JxemallDistrictFilter {
    All,
    Codes(Vec<String>),
}

impl JxemallDistrictFilter {
    pub fn to_payload_lists(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::All => (vec![], vec![]),
            // 当前定版：地区筛选仅使用 district_code_list；administrative_district_code_list 固定空数组。
            Self::Codes(codes) => (codes.clone(), vec![]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JxemallDistrictCode(pub String);

impl From<&str> for JxemallDistrictCode {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

pub type JxemallDistrictCodeNameMap = std::collections::BTreeMap<JxemallDistrictCode, String>;

pub trait JxemallDistrictCodeLookup {
    fn district_name(&self, code: &JxemallDistrictCode) -> Option<&str> {
        self.get(code).map(String::as_str)
    }
}

impl JxemallDistrictCodeLookup for JxemallDistrictCodeNameMap {}

impl JxemallListNewestPayload {
    // 竞价模块无筛选默认模板（业务实例筛选项完整版本）。
    pub fn bidding_default() -> Self {
        Self {
            back_category_name: String::new(),
            trade_model: JxemallTradeModel::Bidding,
            category_type: None,
            page_no: 1,
            page_size: 16,
            state_list: JxemallBiddingStateFilter::all_codes(),
            other_search: String::new(),
            min_budget: None,
            max_budget: None,
            instance_codes: JxemallInstanceCode::all_in_ui_order(),
            sort_field: JxemallSortField::AnnouncementPublishTime,
            sort_method: JxemallSortMethod::Desc,
            district_code_list: vec![],
            administrative_district_code_list: vec![],
        }
    }

    // 反拍模块无筛选默认模板（instance_codes 固定为 JXWSCS）。
    pub fn reverse_default() -> Self {
        Self {
            back_category_name: String::new(),
            trade_model: JxemallTradeModel::Reverse,
            category_type: None,
            page_no: 1,
            page_size: 16,
            state_list: JxemallBiddingStateFilter::all_codes(),
            other_search: String::new(),
            min_budget: None,
            max_budget: None,
            instance_codes: vec![JxemallInstanceCode::OnlineSupermarket],
            sort_field: JxemallSortField::AnnouncementPublishTime,
            sort_method: JxemallSortMethod::Desc,
            district_code_list: vec![],
            administrative_district_code_list: vec![],
        }
    }
}
```

说明：
- `StartCrawlReq.filters` 当前用于兼容多站点透传。
- 对 jxemall 实现时建议在 crawler 内部将 `filters` 反序列化为 `JxemallListNewestPayload` 做强类型校验。
- `category_type = None` 表示“全部类目”。
- `category_type = None` 在序列化时会输出 `"categoryType": null`，用于表达“全部类目”。
- `min_budget`/`max_budget` 使用 `skip_serializing_if = "Option::is_none"`，在“全部”场景省略字段。
- `state_list` 建议由 `JxemallBiddingStateFilter` 统一生成，避免业务层直接散落状态码常量。
- `instance_codes` 为固定枚举集合（`JXWSCS/JXDDCG/JXXYGH/JXFWGC`），建议通过枚举构造，避免手写字符串。
- `district_code_list` 承载地区筛选值；`administrative_district_code_list` 当前固定为空数组。
- 注意区分“不过滤地区”(空数组) 与 “全选地区”(完整代码集) 两种语义。
- `min_budget`/`max_budget` 建议由 `JxemallBudgetFilter` 统一映射生成。
- `sort_field`/`sort_method` 建议使用强类型枚举，避免字符串拼写错误。
- `trade_model` 建议使用强类型枚举，当前已知值为 `Bidding` / `Reverse`。
- 当前口径：REVERSE 的 `instance_codes` 固定为 `JXWSCS`。
- `state_list` 与 `category_type` 在 BIDDING/REVERSE 两模块中按同构规则处理（全状态、类目 null 表示全部）。
- `instance_codes` 编码集合本身固定（`JXWSCS/JXDDCG/JXXYGH/JXFWGC`）；其中 REVERSE 当前实现固定使用 `JXWSCS`。
- 地区编码建议先转换为 `JxemallDistrictCode`，再通过 `JxemallDistrictCodeLookup` 取名；这样可以同时兼容平台码与标准行政码语义。
- `JxemallDistrictCodeNameMap` 建议由 `docs/jxemall-district-code-name-map.json` 反序列化生成，作为运行时地区码表。
- 该 JSON 现在收敛为纯 `code -> name` 映射；Rust 侧可直接读取成 `BTreeMap<JxemallDistrictCode, String>`。

## 8. 应用错误定义

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    CommonInvalidArgument,
    CommonNotFound,
    CommonInternal,

    CrawlerAuthExpired,
    CrawlerRateLimited,
    CrawlerSiteChanged,

    StorageConflict,
    StorageWriteFailed,

    SearchQueryInvalid,
    SearchIndexUnavailable,
}
```

## 9. crate 接口 trait 草案

```rust
use async_trait::async_trait;

#[async_trait]
pub trait CrawlerService {
    async fn start_crawl(&self, req: StartCrawlReq) -> Result<StartCrawlResp, AppError>;
    async fn get_crawl_job(&self, req: GetCrawlJobReq) -> Result<CrawlJob, AppError>;
}

#[async_trait]
pub trait RuleService {
    async fn create_rule(&self, rule: Rule) -> Result<Rule, AppError>;
    async fn update_rule(&self, rule: Rule) -> Result<Rule, AppError>;
    async fn list_rules(&self, paging: Paging) -> Result<Page<Rule>, AppError>;
    async fn delete_rule(&self, rule_id: RuleId) -> Result<(), AppError>;
}

#[async_trait]
pub trait SearchService {
    async fn search_records(&self, req: SearchRecordsReq) -> Result<SearchPage, AppError>;
    async fn get_record_detail(&self, req: GetRecordDetailReq) -> Result<Record, AppError>;
}
```

## 10. Tauri command 签名草案

```rust
#[tauri::command]
async fn save_credential(req: SaveCredentialReq) -> Result<SaveCredentialResp, AppError>;

#[tauri::command]
async fn validate_session(req: ValidateSessionReq) -> Result<ValidateSessionResp, AppError>;

#[tauri::command]
async fn start_crawl(req: StartCrawlReq) -> Result<StartCrawlResp, AppError>;

#[tauri::command]
async fn get_crawl_job(req: GetCrawlJobReq) -> Result<CrawlJob, AppError>;

#[tauri::command]
async fn create_rule(rule: Rule) -> Result<Rule, AppError>;

#[tauri::command]
async fn update_rule(rule: Rule) -> Result<Rule, AppError>;

#[tauri::command]
async fn delete_rule(rule_id: RuleId) -> Result<(), AppError>;

#[tauri::command]
async fn list_rules(paging: Paging) -> Result<Page<Rule>, AppError>;

#[tauri::command]
async fn search_records(req: SearchRecordsReq) -> Result<SearchPage, AppError>;

#[tauri::command]
async fn get_record_detail(req: GetRecordDetailReq) -> Result<Record, AppError>;
```

## 11. 实施建议

1. 先在 crates/shared 固化模型与错误码，避免各 crate 重复定义。
2. command 层只做参数校验与错误转换，业务逻辑留在 service 层。
3. 先保证字段向后兼容，新增字段统一 optional。
