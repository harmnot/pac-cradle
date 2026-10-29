//! External providers (`backend-architecture.md` §4.6), each behind a trait so tests and later
//! providers can stand in. Only services call them.

pub mod email;
