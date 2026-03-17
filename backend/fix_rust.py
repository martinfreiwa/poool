import re

with open("backend/src/main.rs", "r", encoding="utf-8") as f:
    code = f.read()

# Remove old api_kyc_status and api_kyc_submit block
blocks = [
    (r"async fn api_kyc_status.*?\]\n        \}\n    \}\n\}", ""),
    (r"/// POST /api/kyc/submit.*?    \}\n\}", "")
]

# We will just do textual replacement for the commented route
code = code.replace('// .route("/api/kyc/submit", post(kyc::routes::submit))', '.route("/api/kyc/submit", post(kyc::routes::submit))')

# We can also just remove everything after api_support_tickets_submit to the end of file that matches api_kyc*
# Actually regex is safer.
start_idx = code.find("async fn api_kyc_status")
if start_idx != -1:
    code = code[:start_idx]

with open("backend/src/main.rs", "w", encoding="utf-8") as f:
    f.write(code)

with open("backend/src/email.rs", "r", encoding="utf-8") as f:
    email_code = f.read()
email_code = email_code.replace("use tracing::{info, error};", "use tracing::info;")
email_code = email_code.replace("async fn process_onboarding_drips(pool: &PgPool)", "async fn process_onboarding_drips(_pool: &PgPool)")
email_code = email_code.replace("async fn process_abandoned_carts(pool: &PgPool)", "async fn process_abandoned_carts(_pool: &PgPool)")
email_code = email_code.replace("async fn process_win_backs(pool: &PgPool)", "async fn process_win_backs(_pool: &PgPool)")
email_code = email_code.replace("async fn process_milestones(pool: &PgPool)", "async fn process_milestones(_pool: &PgPool)")
email_code = email_code.replace("pub async fn trigger_transactional_email(pool: &PgPool,", "pub async fn trigger_transactional_email(_pool: &PgPool,")
with open("backend/src/email.rs", "w", encoding="utf-8") as f:
    f.write(email_code)

