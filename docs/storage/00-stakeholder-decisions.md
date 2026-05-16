# Storage Audit — Stakeholder Decisions

Block-list of questions that **must be answered** before / during the
storage-subsystem hardening pass can complete. Each comes with a
recommended default the implementation will assume unless overridden.

Update the **Decision** column in-place. Sign-off (initial + date) at
the bottom locks the implementation against the recorded values.

---

## Q1 — KYC-Bucket Region

**Question**: KYC documents currently live in `gs://poool-assets-primary` which the roadmap places in `asia-southeast1` (Singapur). DACH-Nutzer-KYC = Drittlandsübermittlung (DSGVO Art. 44-50). Migrate?

| Option | Pros | Cons |
|---|---|---|
| A — Migrate to `europe-west3` (Frankfurt) | DSGVO-konform, niedrige Latency für DACH-Users, deutsche Datenhoheit | Migration-Effort (~3h), Bucket-Rename, alle existing `gs://` Pfade in DB müssen rewritten |
| B — Keep `asia-southeast1`, add SCC + TIA | No migration effort | TIA + SCC = ~2 Wochen Legal-Aufwand, fragile Compliance-Position post-Schrems II |
| C — Dual: KYC in EU, public assets in asia | Best of both | 2 Buckets statt 1, doppelte IAM-Konfig |

**Recommended Default**: **C — Dual-Bucket** (`poool-kyc-eu` in `europe-west3` für PII-class-A, `poool-assets-primary` bleibt in asia-se1 für public assets). Migration-Job kopiert + DB-Update + verify-checksum, dann Old-Object-Delete.

**Decision**: ___________________________________

---

## Q2 — Antivirus / Malware-Scanning Provider

**Question**: User-uploaded PDFs/Office-docs/zips müssen vor Speicherung gescannt werden (BSI C5 RB-13, ISO 27001 A.12.2.1). Welcher Provider?

| Option | Pros | Cons | Cost/Mo |
|---|---|---|---|
| A — Cloud Function + ClamAV (self-hosted) | Free, full control, no PII zu Drittanbieter | Updates (signatures) self-managed, ~256MB RAM Cold-Start | $0-5 |
| B — VirusTotal API | Best-of-breed multi-engine, 70+ scanners | PII potenziell zu VT exposed, Free-Tier = 4 req/min | $0-200 |
| C — Google Cloud Web Risk + ClamAV combo | Native GCP integration, no extra Provider | Same Cold-Start als A | $0-10 |
| D — Skip AV initially, enable post-launch | Phase 1 ships faster | Akute Vulnerability, BAIT-Verstoß bis enabled | $0 |

**Recommended Default**: **A — Cloud Function + ClamAV**, triggered on every KYC + asset-doc upload via Eventarc. Quarantine bucket für detected threats, Sentry-Alert + auto-reject + audit_logs entry.

**Decision**: ___________________________________

---

## Q3 — CMEK (Customer-Managed Encryption Keys via Cloud KMS)

**Question**: KYC + Asset-Doc Buckets nutzen aktuell Google-managed default AES-256. BaFin / Banken-Customers fordern oft CMEK. Aktivieren?

| Option | Pros | Cons | Cost/Mo |
|---|---|---|---|
| A — Enable CMEK für KYC-Bucket only | Industrie-Standard für PII-class-A, klare Audit-Story | Cloud KMS keys ($0.06/key/month + $0.03/10k operations) | ~$5 |
| B — CMEK für alle Buckets | Maximaler Schutz | Höhere Cost, mehr Operational Overhead (Key-Rotation) | ~$15 |
| C — Keep default Google-managed | Zero-effort, BAIT akzeptiert technisch | Weniger "enterprise-grade" Marketing-Story | $0 |

**Recommended Default**: **A — KYC-Bucket only**. Public Assets brauchen kein CMEK (sind eh öffentlich). Annual Key-Rotation via Cloud KMS auto-rotation.

**Decision**: ___________________________________

---

## Q4 — Resumable Uploads

**Question**: Bei großen Asset-Docs (20 MB) verliert User auf flaky connection die ganze Upload. GCS supports resumable uploads. Frontend-effort?

| Option | Pros | Cons | Effort |
|---|---|---|---|
| A — Implement resumable für Asset-Docs (>5MB threshold) | Robuste UX, weniger failed-upload Tickets | FE-Library oder eigene `XHR` resumable-impl | 6h |
| B — Defer, behalte current single-shot | Phase 1 ships faster | UX-Lücke bei Flaky-Networks (mobile, low-signal) | 0h |
| C — Drittpartei (uppy.io) embedden | Best UX, schnelle Integration | Library-Dep, größerer JS-Bundle | 4h |

**Recommended Default**: **B — Defer Phase 1**, Resumable kommt in Phase 7 als Nice-to-Have nach Production-Launch.

**Decision**: ___________________________________

---

## Q5 — Multi-Tenant Bucket-Layout

**Question**: Aktuell ein Bucket für alles (`poool-assets-primary`). Aufteilen?

| Option | Pros | Cons |
|---|---|---|
| A — 1 Bucket pro Klasse (kyc, assets, avatars, posts) | Per-Klasse IAM, per-Klasse Lifecycle, per-Klasse Retention | 4 Buckets zu konfigurieren, Cross-Bucket-Move bei Misklassifikation |
| B — 2 Buckets (private vs public) | Einfach + privacy-trennung | KYC + Asset-Docs sharen Lifecycle (kann ok sein) |
| C — Single Bucket mit Folder-Prefixes (current) | Simpel | IAM nur via Object-Path-Prefix-Conditions (komplexer als per-Bucket) |

**Recommended Default**: **B — 2 Buckets**: `poool-public-eu` (Avatare, Logos, Post-Images, Asset-Images) + `poool-private-eu` (KYC + Asset-Docs). Privacy-Trennung physisch, einfaches IAM.

**Decision**: ___________________________________

---

## Q6 — Cross-Region Backup-Destination

**Question**: GCS-Backup-Bucket wo replizieren?

| Option | Pros | Cons | Cost/Mo per TB |
|---|---|---|---|
| A — `europe-north1` (Finland, near EU but ferne DR) | Distance ~1500km, EU-Datenhoheit | Slightly higher egress to EU-West | ~$24 |
| B — `us-central1` | Geographic max-distance | Drittland (DSGVO!) bei KYC-Bucket → braucht SCC | ~$20 |
| C — Multi-region `eu` Bucket (auto-replicated) | Native GCS feature, no Cron | Multi-region ist Premium-tier (~$26/TB), nicht Coldline-fähig | ~$26 |

**Recommended Default**: **A — `europe-north1`** für Backup-Bucket, daily `gsutil rsync` via Cloud Scheduler. Coldline storage-class für Backup-Objects (Cost $0.004/GB/month).

**Decision**: ___________________________________

---

## Q7 — KYC-Retention exakt

**Question**: GwG §8 verlangt 5-10 Jahre Aufbewahrung. Trigger?

| Option | Pros | Cons |
|---|---|---|
| A — 5 Jahre nach KYC-Approval | Einfacher Trigger | User mit aktiver Business-Beziehung verlieren Docs zu früh |
| B — 5 Jahre nach Business-Beziehungs-Ende (account-deletion oder 24-Monate-Inaktivität) | GwG-konform per Buchstabe des Gesetzes | Komplexer Trigger, "Beziehungs-Ende" muss exact definiert sein |
| C — 10 Jahre nach KYC-Approval (sicherer Puffer) | Über-konservativ, kein Compliance-Risiko | Cost (Storage) + DSGVO-Min-Speicher-Verstoß möglich (über-retention) |

**Recommended Default**: **B — 5 Jahre nach Business-Beziehungs-Ende**, Cloud Scheduler täglich prüft `users.deleted_at IS NOT NULL AND last_activity_at < NOW() - 24 MONTHS THEN start_5y_clock`.

**Decision**: ___________________________________

---

## Q8 — DSGVO User-Delete Workflow

**Question**: Was passiert mit KYC-Doc wenn User Account löschen will (DSGVO Art. 17)?

| Option | Pros | Cons |
|---|---|---|
| A — Hard-Delete sofort (DB + GCS) | DSGVO Art. 17 maximal erfüllt | KYC-Audit-Trail weg, GwG-Konflikt wenn Account < 5y alt |
| B — Anonymize-and-Keep (PII removed, doc-hash + metadata retained) | Beide Gesetze kompatibel | Implementation komplex (PII-redaction in PDFs) |
| C — Erkläre User "Recht-auf-Vergessen wird nach GwG-Frist erfüllt" + soft-delete jetzt | Rechtssichere Position, kein Rechtsstreit | UX-friction beim Account-Delete |

**Recommended Default**: **C — Soft-Delete + GwG-Erläuterung**. `kyc_documents.deleted_at = NOW(), visibility = 'archived'` sofort. Hard-Delete + GCS-Object-Delete erst nach Retention-Ablauf (Q7 Antwort).

**Decision**: ___________________________________

---

## Sign-Off

| Role | Name | Date | Signature |
|---|---|---|---|
| Product Owner | | | |
| Legal / Compliance | | | |
| CTO / Engineering | | | |

Once signed, this document becomes the implementation-truth-source for the storage hardening phases 1-6. Any deviation requires a new sign-off round.
