/// Community Module (Phase M1)
/// Handles announcements, reactions, comments, and community profiles.
pub mod models;
pub mod routes;
pub mod service;
pub mod user_bridge;
pub mod validation;

pub use routes::router;

#[cfg(test)]
pub mod tests;
