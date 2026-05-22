#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSite {
    Jxemall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub source_id: String,
    pub source_url: String,
    pub title: String,
    pub region_code: String,
    pub published_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CrawlFilter {
    pub district_codes: Vec<String>,
    pub keyword_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlJobRequest {
    pub source: SourceSite,
    pub filter: CrawlFilter,
    pub started_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlJobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlJobProgress {
    pub job_id: String,
    pub status: CrawlJobStatus,
    pub processed: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryRule {
    pub must: Vec<String>,
    pub should: Vec<String>,
    pub must_not: Vec<String>,
    pub minimum_should_match: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRequest {
    pub rule: QueryRule,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub source_id: String,
    pub title: String,
    pub region_code: String,
    pub score: i64,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResponse {
    pub total: u64,
    pub hits: Vec<SearchHit>,
}
