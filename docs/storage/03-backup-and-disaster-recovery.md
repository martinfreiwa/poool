# Storage — Backup & Disaster Recovery Runbook

Covers the **third leg** of the storage hardening pass: how to make the
GCS object layer survive accidental deletes, ransomware, region outages,
or a misconfigured IAM role. Paired with the DB backup story in
MASTERPLAN §"Backup-Strategie".

The user requirement is non-negotiable: **"Dateien dürfen wir nicht
verloren gehen"** — Files must not be lost. This document is the
implementation contract for that promise.

> Scope: All buckets that hold business data — KYC, asset documents,
> avatars, post images, asset images, developer logos. Test buckets
> (`*-staging`, `*-ephemeral`) are explicitly out of scope.

---

## Failure Modes We Are Defending Against

| # | Failure | Probability | Blast Radius | Mitigation |
|---|---|---|---|---|
| F1 | Single object deleted (UI bug, accidental admin) | High | 1 file | Object Versioning + 90d non-current retention |
| F2 | Whole bucket emptied (compromised SA key) | Low | All files | Versioning + IAM `storage.objects.delete` revoked from app SA |
| F3 | Region-wide GCS outage | Very Low | Whole bucket | Cross-region replica in `europe-north1` |
| F4 | Ransomware encrypts via app SA | Low | All writable files | Versioning preserves clean prior copies |
| F5 | Bucket deleted (admin error) | Very Low | Whole bucket | Bucket-level `lifecycle.bucket.delete` denied IAM, replica intact |
| F6 | Silent corruption (bit-rot on GCS) | Very Low | 1 file | SHA-256 stored in DB → nightly verify job |
| F7 | DB row deleted but object orphaned (or vice versa) | Medium | 1 file per drift | Nightly reconciler job + Sentry alert |

Findings F1, F4, F6, F7 are addressed by code/config in this document.
F2, F3, F5 are addressed by IAM hardening + the replica strategy.

---

## Layer 1 — Object Versioning (defends F1, F2, F4)

GCS retains a non-current version of every overwritten or deleted object
once Versioning is enabled at the bucket level. Cost is the storage of
the prior bytes for the retention period.

### Enable (one-time, per bucket)

```bash
# Run with an IAM identity that has roles/storage.admin on the bucket.
for B in poool-assets-primary poool-private-eu poool-public-eu; do
  gsutil versioning set on gs://$B
  gsutil versioning get gs://$B   # verify
done
```

### Lifecycle: purge non-current after 90 days

90 days is the longest plausible "we just noticed" window for an
accidental delete and short enough that storage cost stays bounded.
Tune per bucket if KYC retention rules (Q7 decision) require longer.

`lifecycle-non-current-90d.json`:

```json
{
  "lifecycle": {
    "rule": [
      {
        "action": { "type": "Delete" },
        "condition": {
          "daysSinceNoncurrentTime": 90,
          "numNewerVersions": 1
        }
      }
    ]
  }
}
```

Apply:

```bash
gsutil lifecycle set lifecycle-non-current-90d.json gs://poool-assets-primary
gsutil lifecycle get gs://poool-assets-primary    # verify
```

### Recovery procedure (file accidentally deleted)

```bash
# 1. List all versions (current + non-current) of the object.
gsutil ls -a gs://poool-private-eu/kyc/<user_id>/<file>

# 2. Restore the most recent non-current generation by copying it back.
gsutil cp \
  gs://poool-private-eu/kyc/<user_id>/<file>#<generation> \
  gs://poool-private-eu/kyc/<user_id>/<file>

# 3. Verify SHA-256 against the DB row (the dedup column from Phase 1).
gsutil cat gs://poool-private-eu/kyc/<user_id>/<file> | shasum -a 256
psql -c "SELECT content_sha256 FROM kyc_documents WHERE id = '<doc_id>';"
# Both must match. If not, copy a different generation.
```

---

## Layer 2 — Cross-Region Replication (defends F3, F5)

Storage Transfer Service runs as a managed cron job, copying new and
changed objects from the primary bucket to a replica bucket in a
different region. Recommended replica region: **`europe-north1`**
(Finland, ~1500 km from `europe-west3`, EU sovereignty, Coldline-eligible).

### One-time setup

```bash
# 1. Create the replica bucket (Coldline storage class for cost).
gcloud storage buckets create gs://poool-private-eu-replica \
  --location=europe-north1 \
  --default-storage-class=COLDLINE \
  --uniform-bucket-level-access \
  --public-access-prevention

# 2. Enable Versioning on the replica too (defence in depth).
gsutil versioning set on gs://poool-private-eu-replica

# 3. Grant the Storage Transfer Service SA read on source + write on replica.
SOURCE_SA="project-<PROJECT_NUMBER>@storage-transfer-service.iam.gserviceaccount.com"
gsutil iam ch \
  serviceAccount:$SOURCE_SA:roles/storage.objectViewer \
  gs://poool-private-eu
gsutil iam ch \
  serviceAccount:$SOURCE_SA:roles/storage.objectAdmin \
  gs://poool-private-eu-replica

# 4. Create the daily transfer job (creates + delete-marker propagation OFF).
gcloud transfer jobs create \
  gs://poool-private-eu \
  gs://poool-private-eu-replica \
  --name=daily-replica-private-eu \
  --schedule-starts=$(date -u -v+1H '+%Y-%m-%dT%H:00:00Z') \
  --schedule-repeats-every=1d \
  --include-prefixes="kyc/,asset_documents/" \
  --no-delete-from-destination
```

**Why `--no-delete-from-destination`**: If an attacker mass-deletes the
primary, the replica must NOT mirror the delete. The replica is a
write-only audit copy that we re-mirror back manually after incident
review.

### Failover procedure (primary region outage)

```bash
# 1. Confirm outage scope at https://status.cloud.google.com/.
# 2. Repoint the app to the replica bucket (env var GCS_BUCKET).
gcloud run services update poool-backend \
  --update-env-vars GCS_BUCKET=poool-private-eu-replica \
  --region=europe-west3
# 3. The app is now read-only on replica until primary recovers.
#    (Uploads will fail with 503 — accept the degradation.)
# 4. After primary recovers, rsync replica → primary and flip back.
```

### Replica integrity sanity check (weekly)

```bash
# Pick 10 random objects from primary, compare CRC32C against replica.
gsutil ls gs://poool-private-eu/** | shuf -n 10 | while read OBJ; do
  REL=${OBJ#gs://poool-private-eu/}
  P=$(gsutil stat $OBJ | awk '/Hash \(crc32c\)/ {print $3}')
  R=$(gsutil stat gs://poool-private-eu-replica/$REL | awk '/Hash \(crc32c\)/ {print $3}')
  [ "$P" = "$R" ] && echo "OK  $REL" || echo "MISMATCH $REL  P=$P  R=$R"
done
```

---

## Layer 3 — Reconciliation Job (defends F6, F7)

Periodic job that walks every row in `kyc_documents` and `asset_documents`
and asserts:

1. The referenced GCS object still exists.
2. The object's CRC32C / size matches what the DB row recorded at upload
   (Phase 1's `content_sha256` + `content_size_bytes` columns).
3. There is no orphaned GCS object without a DB row pointing at it.

Implemented in `backend/src/storage/reconciler.rs` and exposed via
`POST /api/admin/storage/reconcile` (admin-only) plus a Cloud Scheduler
nightly trigger.

### Output

Each run writes a row to `storage_reconcile_runs` (created by migration
199) with:

- `started_at`, `finished_at`
- `rows_scanned`, `objects_scanned`
- `missing_objects` (DB row exists, GCS object does not)
- `orphan_objects` (GCS object exists, no DB row references it)
- `hash_mismatches` (size or sha256 differ)
- `sentry_event_id` (if any anomaly was reported)

Each individual anomaly is logged to `storage_reconcile_findings` so the
operator has a single SQL query to triage:

```sql
SELECT * FROM storage_reconcile_findings
 WHERE run_id = (SELECT id FROM storage_reconcile_runs
                  ORDER BY started_at DESC LIMIT 1)
 ORDER BY severity DESC;
```

### Triage runbook

| Finding | Likely cause | Action |
|---|---|---|
| `missing_object` | GCS object was deleted (or never uploaded due to silent failure) | Check Object Versioning for a non-current generation; restore. If no version exists, mark `kyc_documents.deleted_at` and notify user to re-upload. |
| `orphan_object` | Upload succeeded but DB INSERT rolled back; or deletion-cascade left the GCS object behind | Delete the orphan if older than 24h (younger = in-flight upload). Audit-log the delete. |
| `hash_mismatch` | Bit-rot, manual tamper, or wrong object served | Block all reads of that object. Restore from Versioning. Open Sentry incident — this is a P0. |

---

## Layer 4 — IAM Hardening (defends F2, F5)

The application service account must **not** be able to:

- Delete the bucket.
- Disable Versioning.
- Modify the Lifecycle policy.

It should be able to:

- Read + Write objects.
- Delete objects (soft-delete via Versioning preserves recovery).

```bash
# Custom role that allows app-level object operations but no bucket admin.
gcloud iam roles create poool_storage_app \
  --project=$PROJECT \
  --title="POOOL Storage App" \
  --permissions=storage.objects.create,storage.objects.delete,storage.objects.get,storage.objects.list,storage.objects.update

# Grant ONLY this role to the Cloud Run SA. Remove any pre-existing
# roles/storage.admin or roles/storage.objectAdmin grants.
gcloud projects add-iam-policy-binding $PROJECT \
  --member=serviceAccount:poool-backend@$PROJECT.iam.gserviceaccount.com \
  --role=projects/$PROJECT/roles/poool_storage_app
```

Bucket-level deny rule for the catch-all `storage.buckets.delete`:

```bash
gcloud resource-manager org-policies set-policy \
  --project=$PROJECT \
  - <<'EOF'
constraint: constraints/storage.preventBucketDeletion
booleanPolicy:
  enforced: true
EOF
```

---

## RPO / RTO Targets

| Tier | RPO (data loss window) | RTO (downtime) | Mechanism |
|---|---|---|---|
| Critical (KYC, asset docs) | ≤ 24h | ≤ 1h | Daily cross-region replica + Versioning |
| Standard (avatars, post images) | ≤ 7d | ≤ 4h | Versioning + weekly replica |
| Ephemeral (drafts, temp) | None (acceptable loss) | N/A | No replica |

These targets drive the schedule of the Storage Transfer jobs above.
If the business needs sub-hour RPO for KYC later, upgrade to Storage
Transfer continuous replication (event-driven, ~minutes lag).

---

## Verification Checklist (pre-launch)

- [ ] Versioning is enabled on every business bucket (`gsutil versioning get`).
- [ ] Lifecycle policy purges non-current after 90 days.
- [ ] Replica bucket exists in `europe-north1` with Versioning enabled.
- [ ] Storage Transfer job runs daily and last run succeeded.
- [ ] Custom IAM role `poool_storage_app` is the only role on the app SA.
- [ ] Bucket-deletion is denied at the org-policy level.
- [ ] Reconciliation job runs nightly and last 7 runs reported 0 anomalies.
- [ ] Restore drill performed: deleted a test object, restored from a
      non-current generation, verified SHA-256 matched DB row.
- [ ] Restore drill performed: simulated region outage by repointing
      env var to the replica bucket, verified app reads succeed.

Each box must be ticked before sign-off. The drill rows produce
attestation evidence required by BSI C5 RB-15 (DR-Tests).
