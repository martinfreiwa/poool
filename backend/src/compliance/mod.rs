//! Ongoing-diligence compliance domain.
//!
//! Hosts the sanctions / PEP re-screening pipeline (P0-2) and the
//! compliance-alerts queue used by both re-screening and the upcoming
//! transaction-monitoring rule engine (P0-1).

pub mod rescreening;
pub mod routes;
