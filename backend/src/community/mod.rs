/// Community Module (Phase M1/M2/M3/M4/M5)
/// Handles announcements, reactions, comments, profiles, XP, circles, and AMAs.
pub mod models;
pub mod moderation;
pub mod routes;
pub mod service;
pub mod background;
pub mod user_bridge;
pub mod validation;
pub mod xp;
pub mod circles;
pub mod amas;

pub use routes::router;

#[cfg(test)]
pub mod tests;
