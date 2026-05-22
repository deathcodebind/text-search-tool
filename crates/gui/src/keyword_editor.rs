use shared::QueryRule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordClause {
    Must,
    Should,
    MustNot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordTermItem {
    pub id: u64,
    pub term: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeywordEditorError {
    EmptyTerm,
    DuplicateInClause,
    ConflictWithMustNot,
    MinimumShouldMatchOutOfRange,
    TermNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeywordEditorState {
    pub name: String,
    pub enabled: bool,
    pub must: Vec<KeywordTermItem>,
    pub should: Vec<KeywordTermItem>,
    pub must_not: Vec<KeywordTermItem>,
    pub minimum_should_match: u32,
    next_id: u64,
}

impl KeywordEditorState {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            must: Vec::new(),
            should: Vec::new(),
            must_not: Vec::new(),
            minimum_should_match: 0,
            next_id: 1,
        }
    }

    pub fn add_term(
        &mut self,
        clause: KeywordClause,
        term: impl Into<String>,
    ) -> Result<u64, KeywordEditorError> {
        let normalized = normalize_term(term.into())?;

        let list = self.list_ref(clause);
        if list.iter().any(|x| x.term.eq_ignore_ascii_case(&normalized)) {
            return Err(KeywordEditorError::DuplicateInClause);
        }

        if matches!(clause, KeywordClause::Must | KeywordClause::Should)
            && self
                .must_not
                .iter()
                .any(|x| x.term.eq_ignore_ascii_case(&normalized))
        {
            return Err(KeywordEditorError::ConflictWithMustNot);
        }

        if matches!(clause, KeywordClause::MustNot)
            && (self
                .must
                .iter()
                .any(|x| x.term.eq_ignore_ascii_case(&normalized))
                || self
                    .should
                    .iter()
                    .any(|x| x.term.eq_ignore_ascii_case(&normalized)))
        {
            return Err(KeywordEditorError::ConflictWithMustNot);
        }

        let id = self.next_id;
        self.next_id += 1;

        let target = self.list_mut(clause);
        target.push(KeywordTermItem {
            id,
            term: normalized,
        });

        self.validate_minimum_should_match()?;
        Ok(id)
    }

    pub fn update_term(
        &mut self,
        clause: KeywordClause,
        id: u64,
        term: impl Into<String>,
    ) -> Result<(), KeywordEditorError> {
        let normalized = normalize_term(term.into())?;

        let current_list = self.list_ref(clause);
        let Some(index) = current_list.iter().position(|x| x.id == id) else {
            return Err(KeywordEditorError::TermNotFound);
        };

        if current_list
            .iter()
            .enumerate()
            .any(|(idx, x)| idx != index && x.term.eq_ignore_ascii_case(&normalized))
        {
            return Err(KeywordEditorError::DuplicateInClause);
        }

        if matches!(clause, KeywordClause::Must | KeywordClause::Should)
            && self
                .must_not
                .iter()
                .any(|x| x.term.eq_ignore_ascii_case(&normalized))
        {
            return Err(KeywordEditorError::ConflictWithMustNot);
        }

        if matches!(clause, KeywordClause::MustNot)
            && (self
                .must
                .iter()
                .any(|x| x.term.eq_ignore_ascii_case(&normalized))
                || self
                    .should
                    .iter()
                    .any(|x| x.term.eq_ignore_ascii_case(&normalized)))
        {
            return Err(KeywordEditorError::ConflictWithMustNot);
        }

        let target = self.list_mut(clause);
        target[index].term = normalized;
        Ok(())
    }

    pub fn remove_term(&mut self, clause: KeywordClause, id: u64) -> Result<(), KeywordEditorError> {
        let target = self.list_mut(clause);
        let before = target.len();
        target.retain(|x| x.id != id);
        if target.len() == before {
            return Err(KeywordEditorError::TermNotFound);
        }

        if matches!(clause, KeywordClause::Should)
            && self.minimum_should_match > self.should.len() as u32
        {
            self.minimum_should_match = self.should.len() as u32;
        }

        Ok(())
    }

    pub fn set_minimum_should_match(&mut self, value: u32) -> Result<(), KeywordEditorError> {
        self.minimum_should_match = value;
        self.validate_minimum_should_match()
    }

    pub fn to_query_rule(&self) -> Result<QueryRule, KeywordEditorError> {
        self.validate_minimum_should_match()?;

        Ok(QueryRule {
            must: self.must.iter().map(|x| x.term.clone()).collect(),
            should: self.should.iter().map(|x| x.term.clone()).collect(),
            must_not: self.must_not.iter().map(|x| x.term.clone()).collect(),
            minimum_should_match: self.minimum_should_match,
        })
    }

    fn validate_minimum_should_match(&self) -> Result<(), KeywordEditorError> {
        if self.minimum_should_match > self.should.len() as u32 {
            return Err(KeywordEditorError::MinimumShouldMatchOutOfRange);
        }
        Ok(())
    }

    fn list_ref(&self, clause: KeywordClause) -> &Vec<KeywordTermItem> {
        match clause {
            KeywordClause::Must => &self.must,
            KeywordClause::Should => &self.should,
            KeywordClause::MustNot => &self.must_not,
        }
    }

    fn list_mut(&mut self, clause: KeywordClause) -> &mut Vec<KeywordTermItem> {
        match clause {
            KeywordClause::Must => &mut self.must,
            KeywordClause::Should => &mut self.should,
            KeywordClause::MustNot => &mut self.must_not,
        }
    }
}

fn normalize_term(term: String) -> Result<String, KeywordEditorError> {
    let normalized = term.trim().to_string();
    if normalized.is_empty() {
        return Err(KeywordEditorError::EmptyTerm);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{KeywordClause, KeywordEditorError, KeywordEditorState};

    #[test]
    fn should_add_and_export_query_rule() {
        let mut state = KeywordEditorState::new("弱电项目筛选");
        state
            .add_term(KeywordClause::Must, "弱电")
            .expect("must term should be added");
        state
            .add_term(KeywordClause::Should, "安防")
            .expect("should term should be added");
        state
            .add_term(KeywordClause::MustNot, "家具")
            .expect("must_not term should be added");
        state
            .set_minimum_should_match(1)
            .expect("minimum_should_match should be valid");

        let rule = state.to_query_rule().expect("rule export should succeed");
        assert_eq!(rule.must, vec!["弱电"]);
        assert_eq!(rule.should, vec!["安防"]);
        assert_eq!(rule.must_not, vec!["家具"]);
        assert_eq!(rule.minimum_should_match, 1);
    }

    #[test]
    fn should_reject_duplicate_terms_in_same_clause() {
        let mut state = KeywordEditorState::new("dup test");
        state
            .add_term(KeywordClause::Must, "弱电")
            .expect("first add should succeed");

        let err = state
            .add_term(KeywordClause::Must, "弱电")
            .expect_err("duplicate term should fail");
        assert_eq!(err, KeywordEditorError::DuplicateInClause);
    }

    #[test]
    fn should_reject_conflict_between_positive_and_must_not_clauses() {
        let mut state = KeywordEditorState::new("conflict test");
        state
            .add_term(KeywordClause::Must, "弱电")
            .expect("must add should succeed");

        let err = state
            .add_term(KeywordClause::MustNot, "弱电")
            .expect_err("conflict should fail");
        assert_eq!(err, KeywordEditorError::ConflictWithMustNot);
    }

    #[test]
    fn minimum_should_match_should_not_exceed_should_count() {
        let mut state = KeywordEditorState::new("msm test");
        state
            .add_term(KeywordClause::Should, "安防")
            .expect("should add should succeed");

        let err = state
            .set_minimum_should_match(2)
            .expect_err("out-of-range minimum_should_match should fail");
        assert_eq!(err, KeywordEditorError::MinimumShouldMatchOutOfRange);
    }

    #[test]
    fn removing_should_term_should_adjust_minimum_should_match() {
        let mut state = KeywordEditorState::new("remove test");
        let should_id = state
            .add_term(KeywordClause::Should, "安防")
            .expect("should add should succeed");
        state
            .set_minimum_should_match(1)
            .expect("minimum_should_match should be valid");

        state
            .remove_term(KeywordClause::Should, should_id)
            .expect("remove should term should succeed");

        assert_eq!(state.minimum_should_match, 0);
    }
}
