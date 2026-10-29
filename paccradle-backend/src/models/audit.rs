//! The log of admin writes (the `admin_audit_log` table).

use uuid::Uuid;

/// An admin's write request, recorded before it runs.
#[derive(Debug, Clone)]
pub struct AdminWrite {
    /// The entry's id.
    pub id: Uuid,
    /// Who: the admin.
    pub admin_id: Uuid,
    /// Their session.
    pub session_id: Uuid,
    /// The request's `x-request-id`, to find its log lines.
    pub request_id: String,
    /// What: the method…
    pub method: String,
    /// …the route as written in the router, when one matched…
    pub route: Option<String>,
    /// …and the path as called.
    pub path: String,
}
