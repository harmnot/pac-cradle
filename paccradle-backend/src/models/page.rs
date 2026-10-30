//! Paging through a list (every list endpoint: `page`, `page_size`, `total_pages`).

use serde::{Deserialize, Serialize};

/// The largest page a list returns.
pub const MAX_PAGE_SIZE: u32 = 100;
/// The page size when none is asked for.
pub const DEFAULT_PAGE_SIZE: u32 = 20;

/// Which page of a list, counted from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageRequest {
    /// 1 for the first page.
    pub page: u32,
    /// Items per page, 1 to [`MAX_PAGE_SIZE`].
    pub page_size: u32,
}

impl PageRequest {
    /// How many items come before this page.
    pub fn offset(self) -> i64 {
        i64::from(self.page.saturating_sub(1)) * i64::from(self.page_size)
    }

    /// The page size, for SQL `LIMIT`.
    pub fn limit(self) -> i64 {
        i64::from(self.page_size)
    }
}

/// One page of a list and how long the whole list is.
#[derive(Debug, Clone, PartialEq)]
pub struct Page<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// The page asked for.
    pub request: PageRequest,
    /// Items in the whole list, filters applied.
    pub total: u64,
}

impl<T> Page<T> {
    /// Pages in the whole list: 0 when it is empty.
    pub fn total_pages(&self) -> u64 {
        self.total
            .div_ceil(u64::from(self.request.page_size.max(1)))
    }

    /// The same page with each item turned into another type.
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> Page<U> {
        Page {
            items: self.items.into_iter().map(f).collect(),
            request: self.request,
            total: self.total,
        }
    }
}

/// Which way a list is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Smallest, earliest or A first.
    Asc,
    /// Largest, latest or Z first.
    #[default]
    Desc,
}

impl SortOrder {
    /// `asc` or `desc`, as the SQL compares it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}
