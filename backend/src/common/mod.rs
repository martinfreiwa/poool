pub mod audit;
pub mod currency;
pub mod email;
/// Resend → email_logs / email_suppressions webhook receiver.
pub mod email_webhooks;
pub mod idempotency;
/// Safe JSON serialisation for `<script type="application/json">` islands.
pub mod json_safe;
pub mod leader;
pub mod name_match;
pub mod net;
pub mod notifications;
pub mod routes_helper;
pub mod sanitize;
/// Common utilities shared across all domains.
pub mod validation;

#[cfg(test)]
mod financial_tests;

#[cfg(test)]
mod reconciliation_tests;
