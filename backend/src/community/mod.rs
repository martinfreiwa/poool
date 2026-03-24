pub mod amas;
pub mod audit;
pub mod background;
pub mod challenges;
pub mod circles;
/// Community Module (Phase M1/M2/M3/M4/M5)
/// Handles announcements, reactions, comments, profiles, XP, circles, and AMAs.
pub mod models;
pub mod moderation;
pub mod notifications;
pub mod reviews;
pub mod routes;
pub mod service;
pub mod user_bridge;
pub mod validation;
pub mod xp;

pub use routes::router;

#[cfg(test)]
pub mod tests;
