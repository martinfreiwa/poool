# Storage Subsystem — Architecture Overview

Master document tying together the 6-phase storage hardening pass.
Each phase has its own runbook; this file is the **map**.

---

## Component Diagram

```
                       ┌──────────────────────────────────┐
                       │  Browser / Mobile / API client   │
                       └────────────┬─────────────────────┘
                                    │  multipart upload
                                    ▼
        ┌─────────────────────────────────────────────────────────┐
        │  Cloud Run (poool-backend)                              │
        │                                                         │
        │  routes.rs                                              │
        │    ├─ rate_limit (Redis-backed, 10/min/user)            │
        │    ├─ MIME sniff + SVG defensive reject                 │
        │    ├─ SHA-256 compute                                   │
        │    ├─ quota check (storage_user_quotas)                 │
        │    └─ upload_private_with_markers ──┐                   │
        │                                     │                   │
        │  service.rs                         │                   │
        │    ├─ build_client (ADC)            │                   │
        │    ├─ upload_public / upload_private│                   │
        │    ├─ av_status / AvStatus enum     │                   │
        │    └─ PiiClass + classification     │                   │
        │                                     │                   │
        │  reconciler.rs   retention.rs       │                   │
        │    └─ admin/storage.rs ─────────────┼────┐              │
        │                                     │    │              │
        │  metrics.rs (Prometheus /metrics)   │    │              │
        └─────────────────────────────────────┼────┼──────────────┘
                                              │    │
                                              ▼    ▼
                  ┌─────────────────────────────────────────┐
                  │ Google Cloud Storage                    │
                  │                                         │
                  │  poool-assets-primary (public)          │
                  │    avatars/, properties/, posts/        │
                  │                                         │
                  │  poool-private-eu (private + KYC)       │
                  │    kyc/, asset_documents/               │
                  │       • x-goog-meta-pii-class = A       │
                  │       • x-goog-meta-retention-trigger   │
                  │       • x-goog-meta-av-status (ClamAV)  │
                  │       • Versioning enabled              │
                  │       • Lifecycle: purge non-current 90d│
                  │                                         │
                  │  poool-private-eu-replica (europe-north1)│
                  │       • Storage Transfer Service daily  │
                  │       • Coldline + Versioning           │
                  └────────┬────────────────────────────────┘
                           │
                           ▼
                  ┌─────────────────────────────────────────┐
                  │ Eventarc trigger →                      │
                  │ Cloud Function (ClamAV)                 │
                  │    • Scans on every object.create       │
                  │    • Writes x-goog-meta-av-status       │
                  │    • Moves infected → quarantine bucket │
                  └─────────────────────────────────────────┘
```

---

## Module Map (Rust)

| File | Lines* | Responsibility |
|---|---|---|
| `backend/src/storage/mod.rs` | 55 | Router composition + module exports |
| `backend/src/storage/routes.rs` | 1300+ | HTTP upload/download handlers, MIME sniff, quota check, audit log writes |
| `backend/src/storage/service.rs` | 700+ | GCS client wrapper, signed URLs, sha256, AvStatus, QuotaClass, PiiClass, upload_private_with_markers |
| `backend/src/storage/reconciler.rs` | 320 | DB↔GCS drift detector, URL parsing, run lifecycle, finding insertion |
| `backend/src/storage/retention.rs` | 220 | GwG §8 worker, arm_retention_for_user, kind_for_gcs_error |
| `backend/src/admin/storage.rs` | 350 | Admin routes: reconcile + retention/run + retention/arm + analytics |
| `backend/src/metrics.rs` (storage section) | 200 | 8 Prometheus metrics + recorders + refresh_storage_gauges |

\* Approximate at the time the audit closed; subject to future drift.

---

## Database Schema (storage-adjacent)

```
users
  business_relationship_ended_at  TIMESTAMPTZ  -- GwG §8 retention trigger
  (status, frozen_at, …)

kyc_documents
  id, user_id, document_type, gcs_path, status, uploaded_at
  content_sha256        CHAR(64)   -- Phase 1 integrity
  content_size_bytes    BIGINT     -- Phase 1 integrity
  uploaded_ip           INET       -- BAIT 8.3 audit
  uploaded_user_agent   TEXT       -- BAIT 8.3 audit
  retention_until       TIMESTAMPTZ -- Phase 4 GwG §8
  deleted_at            TIMESTAMPTZ -- Phase 4 soft-delete
  deletion_reason       TEXT       -- gwg_retention_expired | dsgvo_user_request | admin_purge

asset_documents
  id, asset_id, document_type, file_url, file_size_bytes
  content_sha256        CHAR(64)
  uploaded_ip, uploaded_user_agent
  uploaded_by_user_id   UUID ON DELETE SET NULL

storage_user_quotas (Phase 2)
  user_id × class       UNIQUE
  bytes_used, file_count, updated_at
  CHECK class IN (avatar, post_image, asset_image, asset_document, kyc_document, developer_logo)

storage_reconcile_runs (Phase 3)
  id, started_at, finished_at, source_table, bucket
  rows_scanned, objects_scanned, missing_objects, orphan_objects
  hash_mismatches, size_mismatches, status, note, sentry_event_id

storage_reconcile_findings (Phase 3)
  run_id, source_id, source_table, object_path, kind, severity, detail JSONB
  acknowledged_at, acknowledged_by

kyc_retention_runs (Phase 4)
  id, started_at, finished_at, rows_considered, rows_due, rows_deleted
  gcs_deletes_ok, gcs_deletes_failed, status, note, dry_run
```

Migrations: 197 (integrity columns), 198 (storage_user_quotas),
199 (reconcile audit), 200 (retention machinery).

---

## Data Flow — KYC Upload (the happy path)

```
1.  Client                                       POST /api/upload/kyc
                                                  multipart body, JWT cookie
2.  axum middleware                              ───►
        ├─ http_metrics_middleware (latency)
        └─ session extractor → User

3.  routes::upload_kyc_document                  ─────────────────────────────┐
        ├─ check_storage_rate_limit(state, user, "kyc")                      │
        ├─ extract multipart file_bytes + claimed_mime                       │
        ├─ is_svg_payload? → 400 + Sentry                                    │
        ├─ sniff_mime → mismatch?                                            │
        │     └─ report_mime_mismatch → Sentry warning                       │
        ├─ check_quota_or_reject(pool, user, QuotaClass::KycDocument, bytes) │
        ├─ sha256_hex(bytes)                                                 │
        ├─ service::upload_private_with_markers(                             │
        │     bucket, path, bytes, mime, PiiClass::A, Some(user.id))         │
        │     │                                                              │
        │     └─► GCS multipart upload with custom metadata                  │
        │                                                                    │
        ├─ INSERT kyc_documents (gcs_path, content_sha256,                   │
        │       content_size_bytes, uploaded_ip, uploaded_user_agent, …)     │
        ├─ increment_quota(pool, user, KycDocument, bytes)                   │
        └─ record_storage_upload("kyc_document", "ok", bytes, elapsed)       │
                                                                             │
4.  GCS object.create event → Eventarc                                       │
        └─► Cloud Function (ClamAV)                                          │
              └─ writes x-goog-meta-av-status=clean (or infected)            │
                                                                             │
5.  Client receives 200 + signed read URL (if requested) ◄────────────────────┘
```

---

## Threat Model Summary

Threats addressed by the implementation, in STRIDE form:

| STRIDE | Threat | Mitigation | Phase |
|---|---|---|---|
| **Spoofing** | Forged upload claiming someone else's identity | Session cookie + JWT, user_id from auth | existing |
| **Tampering** | Mid-flight corruption / disk bit-rot | SHA-256 on upload + nightly reconcile size-check; Object Versioning | 1, 3 |
| **Repudiation** | "I never uploaded that doc" | uploaded_ip + uploaded_user_agent + uploaded_at + uploaded_by_user_id + Cloud Audit Log | 1, 4 |
| **Information disclosure** | Direct GCS URL leak | All KYC stored as `gs://` (no public path); signed URLs with `response-content-disposition=attachment` | 2 |
| **Information disclosure** | Container leaks file via local fallback | `is_local_fallback_allowed()` gates dev-only; production fails loudly | 1 |
| **Denial of service** | Storage quota exhaustion | Per-user per-class quota (bytes + file count) | 2 |
| **Denial of service** | Upload-flood | Rate limit 10 req/60s per user per endpoint class | 2 |
| **Elevation of privilege** | Bucket-level admin compromise via app SA | Custom IAM role with object-level perms only; bucket-delete blocked at org-policy level | 3 |
| **Cross-cutting** | Malware in user upload | ClamAV Cloud Function via Eventarc, AvStatus enforced for KYC reads | 2 |
| **Cross-cutting** | XSS via SVG | Defensive SVG reject at KYC + asset_document + asset_image | 2 |
| **Cross-cutting** | Inline PDF render of attacker payload | Content-Disposition: attachment via `generate_signed_url_with_disposition` | 2 |
| **Cross-cutting** | Single-region GCS outage | Daily Storage Transfer Service replica to europe-north1 | 3 |
| **Cross-cutting** | Object accidental delete | Object Versioning + 90d non-current lifecycle | 3 |
| **Cross-cutting** | DSGVO Art. 17 / GwG §8 conflict | arm_kyc_retention_for_user + nightly worker with dry-run mode | 4 |
| **Cross-cutting** | Silent failure invisibility | 8 Prometheus metrics + 7 alert rules + Sentry capture on every GCS error path | 5 |

---

## What is *not* covered (and why)

| Gap | Decision |
|---|---|
| Resumable uploads | Deferred — see Q4 stakeholder doc, Phase 7 |
| EXIF strip | Deferred — needs the `image` crate, Phase 7 |
| CMEK (customer-managed keys) | Stakeholder decision Q3 — recommended A (KYC bucket only) but not enforced yet |
| Cloud DLP scanning | Deferred to Phase 7; classification markers (Phase 4.3) prepare the ground |
| Per-object access logging UI | Cloud Audit Logs → BigQuery is the source of truth; no in-app viewer needed for v1 |
| Live SVG sanitiser (vs the current reject) | Reject is safer; opt-in sanitiser is Phase 7 if a class-C use case appears |
| Orphan-object detection in reconciler | v2 feature; runbook documents the manual `gsutil ls` procedure |

All gaps are listed in `00-stakeholder-decisions.md` or carry a Phase 7
ticket in `IMPLEMENTATION_ROADMAP.md`.

---

## Reading Order

1. `00-stakeholder-decisions.md` — answer Q1-Q8 first.
2. `01-architecture-overview.md` — this file.
3. `02-antivirus-scanning.md` — ClamAV Cloud Function spec.
4. `03-backup-and-disaster-recovery.md` — Versioning, replica, reconciler.
5. `04-compliance-and-retention.md` — GwG §8 + DSGVO workflow + GCS audit logs.
6. `05-observability.md` — metrics, alerts, SLOs.
7. `06-deployment-runbook.md` — exact deploy sequence.
8. `07-acceptance-criteria.md` — sign-off matrix.
