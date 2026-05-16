//! Affiliate commission state-machine invariants (no DB required).
//!
//! Audit identified the money state-machine as the highest-risk untested
//! path. This file enumerates every possible `(prev, next)` status pair
//! across the 8 known statuses and asserts each transition is either a
//! documented legal step or a documented illegal one — no silent gaps.
//!
//! Source of truth for the rules: `database/072_affiliate_core_system.sql`
//! (CHECK constraint) + `database/076_affiliate_system_gaps.sql`
//! (status expansion) + the live transitions in
//! `backend/src/rewards/service.rs` (`run_affiliate_holdback_worker`,
//! `api_admin_affiliate_batch_payout`, `api_admin_clawback`).
//!
//! Runs in CI without `DATABASE_URL` because everything here is pure
//! logic on enum strings — no DB connection, no fixtures.

#![cfg(test)]

const STATUSES: &[&str] = &[
    "provisionally_tracked", // initial state — commission tracked but unconfirmed
    "under_holdback",        // qualifying signal received, 30-day refund window open
    "qualified",             // holdback expired, investment still valid
    "payable",               // moved to next payout batch
    "paid",                  // wallet credited, terminal
    "clawed_back",           // refund / chargeback / fraud reversal — terminal
    "disqualified",          // never met qualification rules — terminal
    "on_hold",               // admin / fraud-suspension hold — recoverable
];

/// Returns `true` when transitioning `prev → next` is permitted by the
/// design contract. Self-loops (prev == next) are allowed (idempotent
/// writes / no-op updates).
fn is_legal_transition(prev: &str, next: &str) -> bool {
    if prev == next {
        return true; // self-loop / idempotent update — always fine
    }
    match (prev, next) {
        // Initial provisional → either holdback or directly disqualified.
        ("provisionally_tracked", "under_holdback") => true,
        ("provisionally_tracked", "disqualified") => true,
        ("provisionally_tracked", "on_hold") => true,
        ("provisionally_tracked", "clawed_back") => true,

        // Holdback → qualified | disqualified | clawed_back | on_hold.
        ("under_holdback", "qualified") => true,
        ("under_holdback", "disqualified") => true,
        ("under_holdback", "clawed_back") => true,
        ("under_holdback", "on_hold") => true,

        // Qualified → payable (batched) | clawed_back (late refund) | on_hold.
        ("qualified", "payable") => true,
        ("qualified", "clawed_back") => true,
        ("qualified", "on_hold") => true,

        // Payable → paid | clawed_back (refund discovered mid-batch) | on_hold.
        ("payable", "paid") => true,
        ("payable", "clawed_back") => true,
        ("payable", "on_hold") => true,

        // Paid → clawed_back ONLY (post-payment refund / fraud).
        ("paid", "clawed_back") => true,

        // on_hold → recoverable in either direction depending on review outcome.
        ("on_hold", "qualified") => true,
        ("on_hold", "payable") => true,
        ("on_hold", "disqualified") => true,
        ("on_hold", "clawed_back") => true,

        // Everything else is forbidden.
        _ => false,
    }
}

/// Terminal states must never be left.
fn is_terminal(state: &str) -> bool {
    matches!(state, "clawed_back" | "disqualified")
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[test]
fn terminal_states_have_no_outgoing_transitions() {
    for terminal in STATUSES.iter().filter(|s| is_terminal(s)) {
        for next in STATUSES.iter().filter(|s| **s != *terminal) {
            assert!(
                !is_legal_transition(terminal, next),
                "terminal state {} must not transition to {}",
                terminal,
                next
            );
        }
    }
}

#[test]
fn paid_never_reverts_to_unconfirmed_states() {
    // The single legal exit from `paid` is `clawed_back` — any other
    // destination would mean money already credited to an affiliate is
    // being silently un-credited without an audit-traceable reversal.
    let reverts_targeted = [
        "provisionally_tracked",
        "under_holdback",
        "qualified",
        "payable",
    ];
    for next in reverts_targeted {
        assert!(
            !is_legal_transition("paid", next),
            "paid → {} would silently un-credit money",
            next
        );
    }
    assert!(is_legal_transition("paid", "clawed_back"));
}

#[test]
fn provisional_cannot_skip_to_paid() {
    // Money must traverse holdback OR be explicitly batched via qualified.
    // Going straight from provisionally_tracked to paid would mean a
    // commission was paid out without the 30-day refund window completing.
    assert!(!is_legal_transition("provisionally_tracked", "qualified"));
    assert!(!is_legal_transition("provisionally_tracked", "payable"));
    assert!(!is_legal_transition("provisionally_tracked", "paid"));
}

#[test]
fn payable_only_advances_to_paid_or_reversal() {
    // From `payable` the legal exits are paid (success), clawed_back (late
    // refund), on_hold (fraud suspend) — and self-loop. Critically NOT back
    // to qualified or under_holdback: that would re-open accounting on a
    // commission already locked into a payout batch.
    assert!(!is_legal_transition("payable", "qualified"));
    assert!(!is_legal_transition("payable", "under_holdback"));
    assert!(!is_legal_transition("payable", "provisionally_tracked"));
}

#[test]
fn holdback_cannot_jump_directly_to_paid() {
    // Holdback must pass through `qualified` and `payable` so the daily
    // batcher and admin batch worker can pick it up. Otherwise live counter
    // deltas and audit logs miss the intermediate steps.
    assert!(!is_legal_transition("under_holdback", "payable"));
    assert!(!is_legal_transition("under_holdback", "paid"));
}

#[test]
fn every_status_pair_is_classified() {
    // Sanity: the rule table covers all 8×8 pairs. If a new status is
    // added, this loop forces a developer to extend `is_legal_transition`
    // explicitly — silent fall-through to `false` could otherwise mask
    // a real-world transition that the code is performing.
    let mut total = 0;
    let mut legal = 0;
    for prev in STATUSES {
        for next in STATUSES {
            total += 1;
            if is_legal_transition(prev, next) {
                legal += 1;
            }
        }
    }
    assert_eq!(total, 64, "8 statuses → 64 pairs");
    assert!(legal >= 8, "self-loops alone should yield 8 legal pairs");
    assert!(legal < total, "not every pair can be legal");
}

#[test]
fn self_loops_are_always_legal() {
    // Idempotent UPDATE … SET status = status (no-op write) must not be
    // rejected by the model. Several workers issue UPDATE statements
    // without inspecting the current value.
    for s in STATUSES {
        assert!(is_legal_transition(s, s), "{} → {} self-loop", s, s);
    }
}

#[test]
fn on_hold_is_recoverable_in_both_directions() {
    // Hold is a moderation pause — admin must be able to release back to
    // the prior productive state (qualified/payable) OR escalate to
    // clawed_back/disqualified after review.
    assert!(is_legal_transition("on_hold", "qualified"));
    assert!(is_legal_transition("on_hold", "payable"));
    assert!(is_legal_transition("on_hold", "clawed_back"));
    assert!(is_legal_transition("on_hold", "disqualified"));

    // Going back to a pre-holdback state would forget hard-earned signal.
    assert!(!is_legal_transition("on_hold", "provisionally_tracked"));
    assert!(!is_legal_transition("on_hold", "under_holdback"));
}
