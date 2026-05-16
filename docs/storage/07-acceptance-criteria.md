# Storage — Acceptance Criteria & Sign-off Matrix

Single source of truth for "storage subsystem is production-ready".
Roll-up of the verification checklists embedded in each phase doc, plus
a production-readiness score and the formal sign-off matrix.

---

## Production-Readiness Score (0-100)

Each capability scores 0/5/10 (none / partial / full). Total = sum.

| # | Capability | Pre-audit | Post-audit | Cap |
|---|---|---:|---:|---:|
| 1 | Upload integrity (sha256 + size + UA + IP on every row) | 0 | 10 | 10 |
| 2 | MIME sniff + magic-byte cross-check | 5 | 10 | 10 |
| 3 | SVG defensive reject (XSS surface) | 0 | 10 | 10 |
| 4 | Per-user storage quota (bytes + file-count) | 0 | 10 | 10 |
| 5 | Rate-limit per upload endpoint | 0 | 10 | 10 |
| 6 | Content-Disposition: attachment on KYC signed URLs | 0 | 10 | 10 |
| 7 | Local-FS fallback gated to dev-only | 0 | 10 | 10 |
| 8 | AV scan hook (Rust enum + Cloud Function spec) | 0 | 5 | 10 |
| 9 | GCS Object Versioning + 90d lifecycle | 0 | 10 | 10 |
| 10 | Cross-region replica (europe-north1) | 0 | 10 | 10 |
| 11 | DB ↔ GCS reconciler (nightly) | 0 | 10 | 10 |
| 12 | KYC retention worker (GwG §8) | 0 | 10 | 10 |
| 13 | DSGVO arm-retention workflow | 0 | 10 | 10 |
| 14 | Classification markers on private uploads | 0 | 10 | 10 |
| 15 | Cloud Audit Logs DATA_READ/WRITE + BigQuery sink | 0 | 5 | 10 |
| 16 | Prometheus metrics + alerts + SLOs | 0 | 10 | 10 |
| 17 | Tracing spans on upload/retention paths | 0 | 5 | 10 |
| 18 | Custom IAM role (no bucket admin on app SA) | 0 | 10 | 10 |
| 19 | Org-policy bucket-deletion deny | 0 | 5 | 10 |
| 20 | DSGVO user-delete UI workflow text | 0 | 10 | 10 |
| **Total** | | **5** | **180/200** | **200** |

Normalised: **90 / 100** (post-audit) vs **2.5 / 100** (pre-audit).

The 20-point gap is by design — items at "5" (partial) require either
infra work the user must perform (8: ClamAV deploy, 15: audit-log
sink) or polish (17: tracing-span field coverage, 19: org-policy
attestation) that can ship after launch without blocking
production-readiness.

---

## Consolidated Verification Checklists

Each row aggregates the per-phase verification list. Cross-references
back to the phase doc for the underlying procedure.

### Phase 1 — Integrity foundation
- [ ] All 4 audit columns present on `kyc_documents` (197 migration applied).
- [ ] All 4 audit columns present on `asset_documents`.
- [ ] First production upload writes non-NULL `content_sha256`.
- [ ] `is_local_fallback_allowed()` returns `false` in production.
- [ ] `cargo test --test storage_phase1_audit` = 12/12 green.

### Phase 2 — Security hardening
- [ ] SVG defensive reject fires on `<svg`-containing payloads.
- [ ] MIME mismatch captures Sentry warning.
- [ ] Quota exceeded returns 400 (not 500).
- [ ] Rate-limit returns 429 after 11th burst request in 60s window.
- [ ] KYC signed URLs include `response-content-disposition=attachment`.
- [ ] `cargo test --test storage_phase2_audit` = 14/14 green.

### Phase 3 — Backup + DR
- [ ] Versioning + Lifecycle confirmed on every business bucket.
- [ ] Replica bucket created in europe-north1, Storage Transfer Service running.
- [ ] Restore drill performed: deleted test object, restored from non-current generation.
- [ ] Failover drill: env-var flip to replica, verified read succeeds.
- [ ] `cargo test --test storage_phase3_audit` = 12/12 green.

### Phase 4 — Compliance + Retention
- [ ] Migration 200 applied (`users.business_relationship_ended_at` + retention columns).
- [ ] `arm_kyc_retention_for_user(uuid, int)` callable.
- [ ] User-delete handler calls `arm_retention_for_user`.
- [ ] Admin off-boarding flow calls `arm_retention_for_user`.
- [ ] Nightly Cloud Scheduler retention job exists.
- [ ] Audit script confirms 100% of `kyc/` objects carry `pii-class=A`.
- [ ] Cloud Audit Logs enabled (DATA_READ + DATA_WRITE + ADMIN_READ).
- [ ] DSGVO-Notice text live in user-delete confirmation modal.
- [ ] `cargo test --test storage_phase4_audit` = 7/7 green.

### Phase 5 — Observability
- [ ] `/metrics` exposes all 8 `storage_*` series.
- [ ] 7 alert rules deployed to Prometheus + `promtool check rules` passes.
- [ ] Grafana dashboard imported with non-zero data within 24 h.
- [ ] At least one synthetic alert reached PagerDuty.
- [ ] `cargo test --test storage_phase5_audit` = 9/9 green.

### Phase 6 — Documentation + deploy
- [ ] All 8 storage docs reviewed by ≥ 2 engineers.
- [ ] Deploy runbook walked through end-to-end in staging.
- [ ] Rollback procedures rehearsed once on staging.
- [ ] Post-deploy 7-day checklist signed off (this file).

---

## Sign-off Matrix

| Role | Name | Date | Signature |
|---|---|---|---|
| Product Owner | | | |
| Engineering Lead | | | |
| Legal / Compliance | | | |
| Security Officer | | | |
| Site Reliability | | | |

**Sign-off authority:**

- **Product Owner** signs off "is fit for purpose" — confirms the
  features land the business need.
- **Engineering Lead** signs off "code quality + test coverage" — owns
  the `cargo test` totals + the architecture doc.
- **Legal / Compliance** signs off `00-stakeholder-decisions.md` Q1, Q7,
  Q8 + the DSGVO workflow text in `04-compliance-and-retention.md`.
- **Security Officer** signs off the threat-model summary in `01-...md`,
  the IAM hardening in `03-...md` §Layer 4, and the AV-spec in `02-...md`.
- **Site Reliability** signs off `05-observability.md` SLOs, alert
  thresholds, and the deploy runbook.

Once all five signatures are recorded, the storage subsystem is
**production-cleared** and the audit closes. Any subsequent change to
the listed capabilities requires a new sign-off round on the impacted
rows only (not the whole matrix).

---

## What Comes After Sign-off

The remaining "5/10" gaps in the score table are tracked as Phase 7
backlog items in `IMPLEMENTATION_ROADMAP.md`:

1. ClamAV Cloud Function deploy + Eventarc wiring (closes capability 8).
2. Cloud Audit Logs → BigQuery sink with 10y retention partition
   (closes capability 15).
3. Tracing-span field coverage audit + `tracing-opentelemetry` bridge
   verification (closes capability 17).
4. Org-policy `storage.preventBucketDeletion` enforced attestation
   (closes capability 19).
5. Resumable uploads (stakeholder Q4 deferred).
6. EXIF strip on image uploads (deferred — needs `image` crate).
7. CMEK on KYC bucket (stakeholder Q3).
8. Cloud DLP scanning of inbound KYC docs (Phase 7).

Each Phase 7 ticket carries its own acceptance criterion. None is a
launch blocker.
