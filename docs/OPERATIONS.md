# Operational Resilience & Disaster Recovery

This document outlines the infrastructure dependencies and recovery protocols for the POOOL Platform.

## 1. Service Map & Dependencies
| Service | Role | Provider |
| :--- | :--- | :--- |
| **PostgreSQL 15** | Primary Data Store (RWA, Users, Ledger) | Google Cloud SQL |
| **Rust Backend** | Core API & Business Logic | Cloud Run |
| **Frontend Assets** | Static UI (HTML/CSS/JS) | Cloud Run (Served via Axum) |
| **Resend** | Transactional & Status Emails | Third-Party API |
| **Cloud Storage** | KYC Documents & Asset Images | Google Cloud Storage |

## 2. Disaster Recovery (DR) Plan

### Database Recovery
- **Snapshot Frequency**: Automated daily backups (30-day retention).
- **Point-in-Time Recovery (PITR)**: Supported up to 7 days via Write-Ahead Logs (WAL).
- **Restoration Steps**:
  1. Login to GCloud Console -> SQL Instances.
  2. Select `poool-db` -> Backups.
  3. Select latest healthy snapshot -> "Restore to new instance".
  4. Update `DATABASE_URL` in backend environment secrets.
  5. **RTO (Recovery Time Objective)**: < 15 minutes.

### Service Failover
- **Cloud Run**: Regionally redundant by default. In case of region failure, redeploy to `europe-west3` (Frankfurt) using CI/CD pipeline.

## 3. Observability & Alerting
Critical thresholds that trigger immediate on-call notification:
- **API Error Rate**: > 1% (5xx responses) over a 5-minute window.
- **Latency**: P95 > 800ms for `/api/admin` or `/api/payments`.
- **Database CPU**: > 80% sustained for 15 minutes.

## 4. Security Incident Response
1. **Compromised Admin Account**: 
   - Immediate action: Run `UPDATE users SET status = 'suspended' WHERE id = 'compromised_id'`.
   - Run `DELETE FROM user_sessions WHERE user_id = 'compromised_id'`.
2. **Data Leak**: Activate internal protocol; inform legal; rotate database credentials immediately.
