use shared::{AppError, CrawlJobProgress, CrawlJobRequest, SearchRequest, SearchResponse};

pub mod login;
pub mod keyword_editor;
pub mod keyword_preview;

pub use login::{
    CrawlerLoginApiClient, LoginApiClient, LoginFieldErrors, LoginForm, LoginPageState,
    LoginRequest, LoginStatus, LoginSuccess,
};
pub use keyword_editor::{
    KeywordClause, KeywordEditorError, KeywordEditorState, KeywordTermItem,
};
pub use keyword_preview::preview_with_editor_state;

pub trait GuiApplication {
    fn run(&self) -> Result<(), AppError>;
    fn trigger_crawl(&self, request: &CrawlJobRequest) -> Result<String, AppError>;
    fn query_progress(&self, job_id: &str) -> Result<CrawlJobProgress, AppError>;
    fn execute_search(&self, request: &SearchRequest) -> Result<SearchResponse, AppError>;
}

#[derive(Debug, Default)]
pub struct GuiApplicationPlaceholder;

impl GuiApplication for GuiApplicationPlaceholder {
    fn run(&self) -> Result<(), AppError> {
        Ok(())
    }

    fn trigger_crawl(&self, _request: &CrawlJobRequest) -> Result<String, AppError> {
        Ok("job-placeholder".to_string())
    }

    fn query_progress(&self, _job_id: &str) -> Result<CrawlJobProgress, AppError> {
        Ok(CrawlJobProgress {
            job_id: "job-placeholder".to_string(),
            status: shared::CrawlJobStatus::Pending,
            processed: 0,
            total: None,
            message: Some("not started".to_string()),
        })
    }

    fn execute_search(&self, _request: &SearchRequest) -> Result<SearchResponse, AppError> {
        Ok(SearchResponse::default())
    }
}
