/// Community Module (Phase M1/M2)
/// Handles announcements, reactions, comments, and community profiles.
pub mod models;
pub mod moderation;
pub mod routes;
pub mod service;
pub mod background;
pub mod user_bridge;
pub mod validation;

pub use routes::router;

#[cfg(test)]
pub mod tests;
