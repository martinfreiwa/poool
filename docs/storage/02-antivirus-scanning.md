# Antivirus / Malware Scanning Pipeline

Production design for KYC + asset-document scanning.

**Status**: Spec + Rust scan-result hook implemented. Cloud Function +
ClamAV runtime is **infrastructure-as-code only** — must be deployed
out-of-band before this hardening item is "done".

Source-of-truth answer for [Q2](./00-stakeholder-decisions.md#q2):
**Option A — Cloud Function + ClamAV**.

---

## Threat model

User-uploaded PDFs / DOC / DOCX / ZIP / images get downloaded by:

1. **Compliance / KYC reviewers** (admin staff) — open in browser PDF viewer
2. **Other authenticated users** — for asset_documents marked `is_investor_visible = true`
3. **Future investor mailings / exports** — bundled in batched delivery

Each downstream consumer is a potential exploit target:

- **PDF-JS exploits** — malicious PDF with embedded JS, executes in admin's browser context
- **Office-macros** — DOCX with auto-exec macros, runs on download-and-open
- **Polyglot files** — file valid as both image and HTML/JS, MIME-sniffed as image but executes as script
- **ZIP bombs** — recursive zip that expands to 100s of GB, DoS on download
- **Embedded threats** — image with embedded executable in EXIF or padded null-bytes

`Content-Disposition: attachment` (Phase 2.1) closes the "inline render"
hole on the admin browser side, but does NOT protect against
download-and-open. AV scan is the only defence.

---

## Architecture

```
┌──────────────┐         ┌──────────────────┐         ┌──────────────────┐
│  Backend     │  PUT    │  poool-private   │ trigger │ Cloud Function   │
│  upload_*    │────────▶│  bucket          │────────▶│ av-scan-clamav   │
└──────────────┘         └──────────────────┘         └────────┬─────────┘
                                                               │
                                ┌──────────────────────────────┘
                                ▼
                         Set object metadata:
                           x-goog-meta-av-status = clean | infected | error
                           x-goog-meta-av-scanner = clamav-1.4.x
                           x-goog-meta-av-scanned-at = ISO-8601
                                │
                                │ if infected: copy to
                                ▼
                         poool-quarantine  +  Sentry alert
```

**Pipeline contract**:

1. Backend uploads to the production bucket (`poool-private-eu` for KYC,
   `poool-assets-primary` for asset docs).
2. GCS Eventarc trigger fires `av-scan-clamav` Cloud Function within
   ~5 seconds of the upload completing.
3. The function downloads the object, runs `clamscan`, and either:
   - **Clean** → sets metadata `x-goog-meta-av-status=clean`, leaves object in place
   - **Infected** → copies to `poool-quarantine` bucket, deletes from prod bucket, sets `x-goog-meta-av-status=infected` on the (now empty) original, **emits Sentry alert**
   - **Scanner error** → sets `x-goog-meta-av-status=error`, leaves object in place, **emits Sentry warning**
4. Every subsequent download via signed-URL checks the metadata before issuing the URL; objects without a `clean` status are blocked at the application layer.

---

## Rust scan-result hook

`storage::service::AvStatus` enum maps the GCS metadata into a typed
result. Callers of `generate_signed_url_with_disposition` should add the
check (NOT YET ENFORCED — wiring waits for the Cloud Function to deploy
so we don't block legitimate uploads on a non-existent scanner).

```rust
pub enum AvStatus {
    Clean,
    Infected(String),  // detection-name
    NotYetScanned,     // queued, no metadata yet
    ScannerError(String),
}

pub async fn av_status(bucket: &str, object_path: &str) -> Result<AvStatus, AppError>;
```

See `storage::service` `av_status` function for the metadata-read
implementation.

---

## Cloud Function deployment

The Cloud Function source lives in a separate repository
(`poool-av-scanner`) so it can be versioned + deployed independently of
the backend. Hand-off summary for the DevOps engineer:

```sh
# 1. Create quarantine bucket with restrictive IAM (only the function SA
#    can write to it; nobody can publicly read).
gcloud storage buckets create gs://poool-quarantine \
  --location=europe-west3 \
  --uniform-bucket-level-access \
  --public-access-prevention=enforced

# 2. Service account for the function.
gcloud iam service-accounts create av-scanner \
  --display-name="POOOL AV Scanner"

# 3. Grant the function read+metadata-write on KYC/asset buckets, read+write on quarantine.
gcloud storage buckets add-iam-policy-binding gs://poool-private-eu \
  --member=serviceAccount:av-scanner@<project>.iam.gserviceaccount.com \
  --role=roles/storage.objectViewer
gcloud storage buckets add-iam-policy-binding gs://poool-private-eu \
  --member=serviceAccount:av-scanner@<project>.iam.gserviceaccount.com \
  --role=roles/storage.objectUser

# 4. Deploy function (source: github.com/POOOL/poool-av-scanner)
gcloud functions deploy av-scan-clamav \
  --gen2 \
  --region=europe-west3 \
  --runtime=python311 \
  --entry-point=scan_object \
  --memory=2Gi \
  --cpu=1 \
  --timeout=300s \
  --service-account=av-scanner@<project>.iam.gserviceaccount.com \
  --trigger-event-filters=type=google.cloud.storage.object.v1.finalized \
  --trigger-event-filters=bucket=poool-private-eu \
  --set-env-vars=QUARANTINE_BUCKET=poool-quarantine,SENTRY_DSN=$AV_SENTRY_DSN

# 5. Repeat for poool-assets-primary (asset_documents bucket)
```

**Memory: 2 GiB** is the minimum for ClamAV with current signature
database (~700 MB). Smaller allocations OOM on the first scan.

**Timeout: 300s** allows a worst-case 20 MB PDF deep-scan to complete.

---

## Operational runbook

### Alert fires: "AV scan: INFECTED detected"

1. Find the audit_logs row: `SELECT * FROM audit_logs WHERE action LIKE 'kyc_document.uploaded%' AND new_state->>'content_sha256' = '<hash>'`
2. Confirm in quarantine bucket: `gsutil ls gs://poool-quarantine/<original-path>`
3. Notify the actor user via support: "We detected malware in your upload, please scan your device and re-upload from a clean source."
4. If actor is a **developer** (asset_documents), escalate to compliance — could indicate a compromised partner account.

### Alert fires: "AV scan: ScannerError"

1. Check Cloud Function logs: `gcloud functions logs read av-scan-clamav --limit=20`
2. Common cause: ClamAV signature DB stale → redeploy function (which pulls fresh DB during cold-start)
3. Until fixed, ALL uploads to that bucket will be blocked at the signed-URL layer (good fail-safe — never serve unscanned content).

### Manual rescan of a single object

```sh
gcloud functions call av-scan-clamav \
  --data='{"bucket":"poool-private-eu","name":"kyc/<user-id>/<file-id>.pdf"}'
```

---

## Defer / open items

- **Signature DB updates**: currently relies on Cloud Function cold-start fetch. For >1k uploads/day consider a sidecar that pre-pulls.
- **Polyglot detection**: ClamAV catches known malware but not arbitrary polyglot files. Add a second-stage check (magika or similar) for PII-class-A documents.
- **Multi-engine**: VirusTotal API integration as Phase 3 hardening — adds 60+ engines.
