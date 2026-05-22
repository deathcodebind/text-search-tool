use search_engine::SearchService;
use shared::{AppError, ErrorCode, SearchRequest, SearchResponse};

use crate::keyword_editor::KeywordEditorState;

pub fn preview_with_editor_state<S: SearchService>(
    service: &S,
    editor: &KeywordEditorState,
    page: u32,
    page_size: u32,
) -> Result<SearchResponse, AppError> {
    let rule = editor.to_query_rule().map_err(|err| {
        AppError::new(
            ErrorCode::InvalidInput,
            format!("keyword rule is invalid for preview: {err:?}"),
        )
    })?;

    service.search(&SearchRequest {
        rule,
        page,
        page_size,
    })
}

#[cfg(test)]
mod tests {
    use search_engine::{InMemorySearchService, SearchDocument};

    use crate::{KeywordClause, KeywordEditorState, preview_with_editor_state};

    fn doc(id: &str, title: &str, content: &str) -> SearchDocument {
        SearchDocument {
            source_id: id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            region_code: "360103".to_string(),
        }
    }

    #[test]
    fn preview_should_return_hits_for_editor_rule() {
        let service = InMemorySearchService::new(vec![
            doc("1", "弱电改造项目", "安防与综合布线"),
            doc("2", "家具采购", "办公桌椅"),
        ]);

        let mut editor = KeywordEditorState::new("preview");
        editor
            .add_term(KeywordClause::Must, "弱电")
            .expect("must term should be added");
        editor
            .add_term(KeywordClause::MustNot, "家具")
            .expect("must_not term should be added");

        let response =
            preview_with_editor_state(&service, &editor, 1, 10).expect("preview should work");
        assert_eq!(response.total, 1);
        assert_eq!(response.hits[0].source_id, "1");
    }
}
