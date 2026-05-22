#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paging {
    pub page: u32,
    pub page_size: u32,
}

impl Default for Paging {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Paged<T> {
    pub total: u64,
    pub items: Vec<T>,
}
