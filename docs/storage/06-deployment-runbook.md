# Storage — Deployment Runbook

Exact step-by-step sequence to roll the 5-phase storage hardening pass
into production. Every step has a verification gate and a rollback
path. Skipping a verification step is a P1 process violation.

> Total wall-clock estimate: ~3-4 h end-to-end for the **GCP-side**
> setup, +30 min for the Rust deploy. Plan a 2-h maintenance window
> for the cutover, with the rest done as warm-up over the prior week.

---

## Pre-flight (T-7 days)

### P-1 Get stakeholder sign-off on Q1-Q8

Edit `docs/storage/00-stakeholder-decisions.md`. Each `Decision: ___`
must be filled in before the GCP-side setup begins. The bucket region
(Q1) and replica region (Q6) determine which `gsutil` commands you
run; you can't go back without a migration job.

### P-2 Audit current GCP project

```bash
# Make sure the project we deploy into is the right one.
gcloud config get-value project
gcloud auth list

# Inventory existing buckets — anything we'll touch must be known.
gsutil ls
```

### P-3 Create a deploy-day Sentry release

```bash
sentry-cli releases new poool-backend@$(git rev-parse --short HEAD)
```

Every Sentry event raised during the deploy will be tagged with this
release so the post-deploy review can isolate regressions.

---

## Phase 0 — Database migrations (T-3 days, ahead of code deploy)

Migrations 197–200 are **backwards-compatible** (additive columns +
new tables only). Apply them ahead of the code deploy so the new
columns exist before any code references them.

```bash
# Apply against the production DB. Use the connection string from Secret Manager.
PROD_DB_URL=$(gcloud secrets versions access latest --secret=db-url-prod)

for M in 197_storage_integrity_audit_columns \
         198_storage_user_quota \
         199_storage_reconcile_audit \
         200_kyc_retention; do
  echo "─── Applying $M ───"
  psql "$PROD_DB_URL" -f database/${M}.sql
done

# Verification: confirm columns + tables exist.
psql "$PROD_DB_URL" -c "\d kyc_documents"   | grep -E "content_sha256|retention_until|deleted_at"
psql "$PROD_DB_URL" -c "\d asset_documents" | grep "content_sha256"
psql "$PROD_DB_URL" -c "\dt storage_user_quotas"
psql "$PROD_DB_URL" -c "\dt storage_reconcile_runs"
psql "$PROD_DB_URL" -c "\dt kyc_retention_runs"
psql "$PROD_DB_URL" -c "\df arm_kyc_retention_for_user"
```

**Rollback**: each migration is wrapped in `BEGIN/COMMIT` with
`IF NOT EXISTS` guards. To unwind, run the equivalent
`ALTER TABLE ... DROP COLUMN IF EXISTS` + `DROP TABLE IF EXISTS` block
— but only if no code is yet reading the new columns.

---

## Phase 1 — GCS bucket prep (T-1 day)

Order matters: enable Versioning **before** the first write that you
want to be recoverable, and apply the Lifecycle policy **after**
Versioning is on (otherwise the lifecycle has nothing to act on).

### 1.1 Enable Object Versioning

```bash
for B in poool-assets-primary poool-private-eu; do
  gsutil versioning set on gs://$B
done
gsutil versioning get gs://poool-private-eu   # expect: gs://...: Enabled
```

### 1.2 Apply Lifecycle policy

```bash
gsutil lifecycle set docs/storage/lifecycle-non-current-90d.json gs://poool-private-eu
gsutil lifecycle get gs://poool-private-eu | jq .
```

### 1.3 Replica bucket + Storage Transfer Service

```bash
gcloud storage buckets create gs://poool-private-eu-replica \
  --location=europe-north1 --default-storage-class=COLDLINE \
  --uniform-bucket-level-access --public-access-prevention

gsutil versioning set on gs://poool-private-eu-replica

# Grant Transfer Service SA the right roles, then create the job.
# (Exact commands in 03-backup-and-disaster-recovery.md §Layer 2.)
```

### 1.4 Custom IAM role on the app SA

```bash
gcloud iam roles create poool_storage_app --project=$PROJECT \
  --title="POOOL Storage App" \
  --permissions=storage.objects.create,storage.objects.delete,storage.objects.get,storage.objects.list,storage.objects.update

# Bind to the Cloud Run SA, remove pre-existing broader grants.
gcloud projects add-iam-policy-binding $PROJECT \
  --member=serviceAccount:poool-backend@$PROJECT.iam.gserviceaccount.com \
  --role=projects/$PROJECT/roles/poool_storage_app
gcloud projects remove-iam-policy-binding $PROJECT \
  --member=serviceAccount:poool-backend@$PROJECT.iam.gserviceaccount.com \
  --role=roles/storage.objectAdmin || true
```

### 1.5 Cloud Audit Logs (BAIT 8.3)

```bash
# See 04-compliance-and-retention.md §Layer 4 for the audit-config.yaml.
gcloud projects get-iam-policy $PROJECT > policy.yaml
# Merge audit-config.yaml into policy.yaml manually.
gcloud projects set-iam-policy $PROJECT policy.yaml
```

### 1.6 Verification gate

- [ ] `gsutil versioning get` returns `Enabled` on every business bucket.
- [ ] `gsutil lifecycle get` shows the 90d non-current rule.
- [ ] Replica bucket exists in `europe-north1` and Storage Transfer
      job has at least one successful run.
- [ ] App SA holds **only** `poool_storage_app` (no admin-tier role).
- [ ] `gcloud projects get-iam-policy` shows DATA_READ + DATA_WRITE
      audit config for `storage.googleapis.com`.

Any unchecked box blocks the code deploy.

---

## Phase 2 — Code deploy (T-0)

### 2.1 Build + push

```bash
# Run the existing deploy workflow — manual trigger (per user's MEMORY.md
# note: push does NOT auto-deploy).
gh workflow run deploy.yml --field environment=production

# Monitor.
gh run watch
```

### 2.2 Smoke test (within 10 min of deploy)

```bash
# /metrics returns all 8 storage metrics
curl -s https://api.poool.app/metrics | grep -E "^storage_(uploads|upload_bytes|upload_duration|gcs_errors|av_outcomes|retention_due|reconcile_findings_open|quota_used_bytes)" | wc -l
# Expect: 8

# Admin reconciler endpoint smoke (dry):
curl -X POST -H "Cookie: $ADMIN_SESSION" \
  "https://api.poool.app/api/admin/storage/reconcile?source=kyc&note=deploy-smoke"
# Expect: 200, JSON with rows_scanned ≥ 0

# Admin retention endpoint smoke (dry-run):
curl -X POST -H "Cookie: $ADMIN_SESSION" \
  "https://api.poool.app/api/admin/storage/retention/run?dry_run=true&note=deploy-smoke"
# Expect: 200, JSON with status="success", dry_run=true
```

### 2.3 Verification gate

- [ ] `/metrics` exposes all 8 `storage_*` series.
- [ ] First production upload succeeds → `kyc_documents` row carries
      `content_sha256`, `content_size_bytes`, `uploaded_ip`,
      `uploaded_user_agent` (not NULL).
- [ ] First production upload triggers an Eventarc → Cloud Function
      execution; object metadata includes `x-goog-meta-av-status`
      within 30s.
- [ ] Sentry release was tagged and zero P0/P1 events in the first
      30 min post-deploy.

---

## Phase 3 — Scheduler wiring (T+1 day)

After the code is live, register the recurring jobs.

```bash
# Nightly reconciler — 02:00 Berlin
gcloud scheduler jobs create http reconciler-kyc \
  --schedule="0 2 * * *" --time-zone="Europe/Berlin" \
  --uri="https://api.poool.app/api/admin/storage/reconcile?source=kyc" \
  --http-method=POST \
  --oidc-service-account-email=poool-scheduler@$PROJECT.iam.gserviceaccount.com

gcloud scheduler jobs create http reconciler-assets \
  --schedule="30 2 * * *" --time-zone="Europe/Berlin" \
  --uri="https://api.poool.app/api/admin/storage/reconcile?source=assets" \
  --http-method=POST \
  --oidc-service-account-email=poool-scheduler@$PROJECT.iam.gserviceaccount.com

# Nightly retention — 03:00 Berlin, AFTER the reconciler
gcloud scheduler jobs create http retention-worker \
  --schedule="0 3 * * *" --time-zone="Europe/Berlin" \
  --uri="https://api.poool.app/api/admin/storage/retention/run?dry_run=false" \
  --http-method=POST \
  --oidc-service-account-email=poool-scheduler@$PROJECT.iam.gserviceaccount.com

# Hourly retention dry-run for the ops dashboard
gcloud scheduler jobs create http retention-preview \
  --schedule="0 * * * *" --time-zone="Europe/Berlin" \
  --uri="https://api.poool.app/api/admin/storage/retention/run?dry_run=true" \
  --http-method=POST \
  --oidc-service-account-email=poool-scheduler@$PROJECT.iam.gserviceaccount.com
```

Verification:

- [ ] `gcloud scheduler jobs list` shows the 4 new jobs in `ENABLED`
      state.
- [ ] After the first scheduled run, `storage_reconcile_runs` /
      `kyc_retention_runs` each have a `status='success'` row.

---

## Phase 4 — Prometheus alerts (T+2 days)

```bash
# Drop the alert rules into the Prometheus config repo.
cp docs/storage/storage-alerts.yml infra/prometheus/alerts/
promtool check rules infra/prometheus/alerts/storage-alerts.yml
git commit -am "feat(monitoring): add storage subsystem alerts"
git push   # CI reloads Prometheus

# Verify in the Prometheus UI under Status → Rules that all 7 rules
# are loaded and at least one of them has evaluated.
```

Verification:

- [ ] All 7 alert rules visible in Prometheus UI.
- [ ] No rule is in `EvaluationError` state.
- [ ] A synthetic alert (manually inc the AV-infected counter via the
      test endpoint) reaches PagerDuty.

---

## Rollback Procedures

| Failure | Rollback |
|---|---|
| Code regression detected in first 30 min | `gh workflow run deploy.yml --field rollback_to=$LAST_GOOD_SHA` — the migrations stay (additive only, no harm). |
| Cloud Function ClamAV broken | Disable the Eventarc trigger. The app still reads/writes; KYC reads degrade to `AvStatus::NotYetScanned` which is acceptable for v1. |
| Storage Transfer Service backing up the replica | `gcloud transfer jobs run daily-replica-private-eu --pause` and investigate the destination IAM. Primary is unaffected. |
| Versioning accidentally disabled | `gsutil versioning set on gs://...` again. Nothing on the disk path is lost — only the *future* delete-recovery for the disabled window. |
| Retention worker mass-deletes something it shouldn't | Stop the scheduler job. The soft-deleted DB rows are still readable (`deleted_at IS NOT NULL`). GCS objects are recoverable from non-current versions if Versioning was on. |

---

## Post-deploy checklist (T+7 days)

After one week in production:

- [ ] Grafana dashboard shows non-zero data for every storage metric.
- [ ] Every alert rule has evaluated > 100 times with no firing
      (or fired only on synthetic tests).
- [ ] `storage_reconcile_runs` has ≥ 5 consecutive `success` rows
      with 0 findings.
- [ ] `kyc_retention_runs` (dry_run=false) has ≥ 5 `success` rows.
- [ ] No `kyc_documents` row was inserted with NULL `content_sha256`
      after the deploy cutover (`SELECT COUNT(*) FROM kyc_documents
      WHERE content_sha256 IS NULL AND uploaded_at > '<deploy_ts>'`).
- [ ] Sentry shows zero unhandled errors from `storage::*`.
- [ ] Sign off `07-acceptance-criteria.md` and archive the release.

---

## Out-of-Band Operations

| Task | Command |
|---|---|
| Manual reconcile (urgent) | `curl -X POST .../api/admin/storage/reconcile?source=kyc&note="manual %REASON%"` |
| Manual retention dry-run | `curl -X POST .../api/admin/storage/retention/run?dry_run=true` |
| Arm retention on a single user (admin off-boarding) | `curl -X POST .../api/admin/storage/retention/arm/<user_uuid>?years=5` |
| Restore an accidentally deleted object | See 03-backup-and-disaster-recovery.md §Recovery procedure |
| Cross-region failover | See 03-backup-and-disaster-recovery.md §Failover procedure |
| Triage AV-infected upload | See 02-antivirus-scanning.md §Triage |
