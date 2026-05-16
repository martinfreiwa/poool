# Storage — Compliance & Retention Runbook

Covers the **fourth leg** of the storage hardening pass: KYC retention
(GwG §8), DSGVO-konforme Löschung (Art. 17 + Art. 5(1)(e)), GCS
audit-logging (BAIT 8.3), und Classification-Marker für PII-class-A.

Pairs with the implementation in `backend/src/storage/retention.rs` and
migration `database/200_kyc_retention.sql`.

> The regulatory framing is **simultaneously: retain (GwG) and delete
> (DSGVO)**. Both obligations are mandatory. The only way out is exact
> per-document retention deadlines + a worker that deletes the moment
> a deadline expires.

---

## Layer 1 — KYC Retention (GwG §8)

GwG §8 mandates 5 years (default) or 10 years (extended in escalated
risk cases) of retention for KYC identification documents, **counted
from the end of the business relationship**, not from the upload date.

### Trigger: business-relationship-end

The clock starts when one of these events fires:

1. User deletes their account (DSGVO Art. 17 request).
2. Admin off-boards the user (KYC permanently failed, contract
   terminated).
3. Inactivity sweep marks the account `business_relationship_ended_at`
   after >24 months without login + zero open positions.

Each trigger calls the SQL function

```sql
SELECT arm_kyc_retention_for_user('<user_uuid>'::uuid, 5);
--                                                     ^^ retention_years
```

Behaviour:

- Sets `users.business_relationship_ended_at = NOW()` (idempotent — the
  first end-date wins).
- Computes `kyc_documents.retention_until = end + N years` for every
  KYC doc that doesn't have one yet.
- Returns the number of rows updated for the audit log.

### Worker: nightly delete

`backend/src/storage/retention.rs::run_retention_worker(pool, bucket,
dry_run=false, note)` scans `kyc_documents WHERE retention_until <= NOW()
AND deleted_at IS NULL`, batches 1000 per run, deletes the GCS object
first (so a GCS failure doesn't orphan a real file), then soft-deletes
the row with `deletion_reason = 'gwg_retention_expired'`. Audit row in
`kyc_retention_runs`.

Schedule:

```bash
# Cloud Scheduler — daily 03:00 Europe/Berlin
gcloud scheduler jobs create http kyc-retention-worker \
  --schedule="0 3 * * *" \
  --time-zone="Europe/Berlin" \
  --uri="https://api.poool.app/api/admin/storage/retention/run" \
  --http-method=POST \
  --oidc-service-account-email=poool-scheduler@PROJECT.iam.gserviceaccount.com \
  --oidc-token-audience="https://api.poool.app/api/admin/storage/retention/run"
```

### Dry-run + observability

`dry_run=true` returns the same summary without touching anything.
The ops dashboard calls it once an hour to surface "how many docs are
due in the next 7 / 30 / 90 days" so we have a forward window.

### Triage runbook

| Symptom | Cause | Action |
|---|---|---|
| `gcs_deletes_failed > 0` | Object 404 / auth lapse / network blip | Reconciler will catch the orphan; rerun worker; verify SA IAM still has `storage.objects.delete`. |
| `rows_due > 1000` for several days | Worker batch cap (1000/run) | Either temporary backlog or end-of-month spike — bump the daily cron to every 6 h until backlog clears. |
| `rows_considered = 0` | Either healthy (no users have ended business relationship) or `arm_kyc_retention_for_user` is not being called from the user-delete flow | Spot-check: pick a deleted user, verify `business_relationship_ended_at IS NOT NULL`. |
| Same row in `gcs_deletes_failed` 3+ runs | Persistent GCS error | Manual investigation — likely IAM revoked or object was already manually deleted. Mark deletion_reason='admin_purge' by hand. |

---

## Layer 2 — DSGVO User-Delete Workflow (Art. 17 + Art. 5(1)(e))

User clicks "Delete my account" → app must:

1. Verify identity (re-authenticate, 2FA challenge).
2. Mark `users.status = 'deleted'`.
3. Call `arm_kyc_retention_for_user(user_id, 5)`.
   - This sets `business_relationship_ended_at`.
   - Computes `retention_until` on every KYC doc.
4. Hard-delete or anonymize all non-KYC PII immediately:
   - `users.email` → `deleted-<uuid>@anonymous.poool.invalid`
   - `users.password_hash` → NULL
   - `users.avatar_url` → NULL + GCS-delete the avatar object now
   - Posts, comments → keep with `author = 'deleted user'` (community
     integrity)
5. Display the legally-required notice: *"Ihre KYC-Dokumente sind nach
   §8 GwG für 5 Jahre nach Beendigung der Geschäftsbeziehung
   aufzubewahren. Sie werden danach vollständig gelöscht. Bei allen
   anderen Daten haben wir Ihrem Löschverlangen sofort entsprochen."*

### Implementation hook

```rust
use crate::storage::retention::arm_retention_for_user;

// In the user-delete handler, AFTER status='deleted' UPDATE:
let kyc_rows_armed = arm_retention_for_user(&pool, user.id, 5).await?;
tracing::info!("DSGVO delete: armed retention on {} KYC docs", kyc_rows_armed);
```

### Edge cases

| Case | Decision |
|---|---|
| User deletes while KYC `status='pending'` | Still arm retention — pending docs must be retained for the audit trail of why they were never approved. |
| User deletes with active investments | Block the delete with a 400. The business relationship can't end while open positions exist. |
| Admin force-deletes a KYC-failed user | Same as user-initiated delete — `arm_retention_for_user(..., 5)`. |
| User re-registers with the same email | Refused. The original email is anonymized + the address is added to `deleted_emails_blacklist` for 5 years. |

---

## Layer 3 — Classification Markers on GCS Objects

Every stored object carries custom metadata that tells the system its
sensitivity class. The reconciler + retention worker + future DLP
scanner all dispatch on this marker rather than guessing from the
object path.

| Marker key | Allowed values | Set by |
|---|---|---|
| `x-goog-meta-pii-class` | `A`, `B`, `C`, `none` | Upload handler |
| `x-goog-meta-retention-trigger` | `business_end+5y`, `business_end+10y`, `none` | Upload handler |
| `x-goog-meta-uploaded-by-user-id` | UUID | Upload handler |
| `x-goog-meta-uploaded-at` | RFC 3339 timestamp | Upload handler |
| `x-goog-meta-av-status` | `clean`, `infected`, `error` | ClamAV Cloud Function (Phase 2.3) |

Classes:

- **A** — Identity documents (KYC). 5y/10y retention, signed URLs only,
  CMEK if enabled, av-scan required.
- **B** — Asset documents (property contracts, financial reports).
  Retention = max(business_end+5y, asset_disposal+5y). Signed URLs.
- **C** — Public assets (avatars, post images, asset thumbnails). No
  retention requirement; deleted with the parent entity.

The upload handler MUST set `pii-class` + `retention-trigger`. A new
upload without these markers is a P1 bug — the reconciler logs a
finding so unmarked objects don't slip into production.

### gcloud check

```bash
# Audit: do all KYC objects carry pii-class=A?
gsutil ls gs://poool-private-eu/kyc/** | while read OBJ; do
  CLASS=$(gsutil stat "$OBJ" | awk '/pii-class:/ {print $2}')
  [ "$CLASS" = "A" ] || echo "UNMARKED $OBJ"
done
```

---

## Layer 4 — GCS Audit Logging (BAIT 8.3)

Enable Cloud Audit Logs **Data Access** for the KYC + asset-doc
buckets. Without this, BaFin-BAIT 8.3 (Auditierbarkeit von
IT-Berechtigungen + Datenzugriffen) is not technically demonstrable.

```bash
# audit-config.yaml
auditConfigs:
- service: storage.googleapis.com
  auditLogConfigs:
  - logType: DATA_READ
  - logType: DATA_WRITE
  - logType: ADMIN_READ
```

Apply at project level (cheapest, covers all buckets):

```bash
gcloud projects get-iam-policy $PROJECT > policy.yaml
# Manually merge audit-config.yaml into policy.yaml
gcloud projects set-iam-policy $PROJECT policy.yaml
```

### Cost expectation

Data Access logs for GCS bill at $0.50/GiB of log volume. With ~10k
uploads/month + 50k reads, expect ~$2-5/month for logs. The
attestation value outweighs the cost by orders of magnitude.

### Log sink → BigQuery for retention

Cloud Logging keeps audit logs for 400 days. BAIT requires 10 years
for material audit events. Sink to BigQuery + Coldline:

```bash
gcloud logging sinks create kyc-audit-sink \
  bigquery.googleapis.com/projects/$PROJECT/datasets/audit_log_warehouse \
  --log-filter='protoPayload.serviceName="storage.googleapis.com"
                AND resource.labels.bucket_name=~"^poool-(private|kyc)-"'
```

Partition the BigQuery table by `timestamp`; Coldline-export
partitions older than 90 days.

---

## Compliance Verification Checklist (pre-launch)

- [ ] Migration 200 applied; `arm_kyc_retention_for_user` SQL function
      callable.
- [ ] User-delete handler calls `arm_retention_for_user`.
- [ ] Admin off-boarding handler calls `arm_retention_for_user`.
- [ ] Inactivity sweep job exists (or accepted gap, documented).
- [ ] Nightly retention worker scheduled via Cloud Scheduler.
- [ ] `kyc_retention_runs.status` history shows ≥7 consecutive
      `success` runs.
- [ ] Upload handlers set `pii-class` + `retention-trigger` markers on
      every KYC + asset-doc upload.
- [ ] Audit script confirms 100% of objects under `kyc/` carry
      `pii-class=A`.
- [ ] Cloud Audit Logs DATA_READ / DATA_WRITE enabled at the project
      level for `storage.googleapis.com`.
- [ ] Log sink → BigQuery exists for ≥10y archival of GCS audit
      events.
- [ ] DSGVO-Notice text rendered in the user-delete confirmation
      modal.
- [ ] Legal sign-off recorded on `00-stakeholder-decisions.md`
      questions Q7 + Q8 + Q6.

Each box must be ticked before sign-off. This file + the run-row
history in `kyc_retention_runs` + the Cloud Audit Logs in BigQuery
are the attestation evidence for BSI C5 ORG-08, BAIT 8.3, and DSGVO
Art. 5(1)(e).
