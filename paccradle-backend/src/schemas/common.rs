use serde::Serialize;
use serde_json::{Map, Value};

use crate::error::{AppError, ErrorCode};
use crate::models::page::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE, Page, PageRequest};

/// Every error response: `{ "error": { "code", "message", "meta" } }`.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// The error.
    pub error: ErrorDetail,
}

/// One error.
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    /// Stable code, e.g. `RATE_LIMITED`.
    pub code: &'static str,
    /// In the caller's language (`Accept-Language`), for showing as is.
    pub message: String,
    /// The values in the message, e.g. `{ "retry_after_seconds": 12 }`. Often empty.
    pub meta: Map<String, Value>,
}

/// One page of a list: `{ items, page, page_size, total, total_pages }` (every list endpoint).
#[derive(Debug, Serialize)]
pub struct PageResponse<T> {
    /// The items on this page.
    pub items: Vec<T>,
    /// The page, counted from 1.
    pub page: u32,
    /// Items per page.
    pub page_size: u32,
    /// Items in the whole list, filters applied.
    pub total: u64,
    /// Pages in the whole list; 0 when it is empty.
    pub total_pages: u64,
}

impl<T> PageResponse<T> {
    /// The response for `page`, each item turned into its response with `item`.
    pub fn new<U>((page, item): (Page<U>, impl FnMut(U) -> T)) -> Self {
        let total_pages = page.total_pages();
        let page = page.map(item);
        Self {
            items: page.items,
            page: page.request.page,
            page_size: page.request.page_size,
            total: page.total,
            total_pages,
        }
    }
}

/// `page` (from 1, default 1) and `page_size` (1 to 100, default 20) from a list's query.
/// Out of range is `INVALID_REQUEST` naming the parameter.
pub fn page_request(
    (page, page_size): (Option<u32>, Option<u32>),
) -> Result<PageRequest, AppError> {
    let page = page.unwrap_or(1);
    if page == 0 {
        return Err(AppError::new(ErrorCode::InvalidRequest)
            .with_meta("field", "page")
            .with_meta("allowed", "1 or more"));
    }
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if !(1..=MAX_PAGE_SIZE).contains(&page_size) {
        return Err(AppError::new(ErrorCode::InvalidRequest)
            .with_meta("field", "page_size")
            .with_meta("allowed", format!("1 to {MAX_PAGE_SIZE}")));
    }
    Ok(PageRequest { page, page_size })
}
