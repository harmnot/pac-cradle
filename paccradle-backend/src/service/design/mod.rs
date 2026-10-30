//! Designs: what the editor saves (M1-D6). Exports (D7) and share links (M2) read them.

mod design_service;
mod design_service_trait;

pub use design_service::{DefaultDesignService, DesignProviders};
pub use design_service_trait::{DesignListRequest, DesignService, DesignUpdate, NewDesignRequest};
