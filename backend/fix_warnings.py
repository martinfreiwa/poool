import os

def replace_in_file(path, replacements):
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()
    
    for old, new in replacements:
        content = content.replace(old, new)
        
    with open(path, "w", encoding="utf-8") as f:
        f.write(content)

# admin/mod.rs
replace_in_file("src/admin/mod.rs", [
    ("use axum::{extract::{State, Json, Path}, response::IntoResponse};", "use axum::{extract::{State, Json}, response::IntoResponse};"),
    ("use serde_json::Value;", ""),
    ("use crate::email;", ""),
])

# email.rs
replace_in_file("src/email.rs", [
    ("use tracing::{info, error};", "use tracing::info;"),
    ("async fn process_onboarding_drips(pool: &PgPool)", "async fn process_onboarding_drips(_pool: &PgPool)"),
    ("async fn process_abandoned_carts(pool: &PgPool)", "async fn process_abandoned_carts(_pool: &PgPool)"),
    ("async fn process_win_backs(pool: &PgPool)", "async fn process_win_backs(_pool: &PgPool)"),
    ("async fn process_milestones(pool: &PgPool)", "async fn process_milestones(_pool: &PgPool)"),
    ("pub async fn trigger_transactional_email(pool: &PgPool,", "pub async fn trigger_transactional_email(_pool: &PgPool,"),
])

# auth/models.rs
replace_in_file("src/auth/models.rs", [
    ("pub struct ResendVerificationForm {}", "#[allow(dead_code)]\npub struct ResendVerificationForm {}"),
])

# auth/service.rs
replace_in_file("src/auth/service.rs", [
    ("pub async fn verify_email(pool: &PgPool, token: &str)", "#[allow(dead_code)]\npub async fn verify_email(pool: &PgPool, token: &str)"),
])

# main.rs
import re
with open("src/main.rs", "r", encoding="utf-8") as f:
    content = f.read()

content = re.sub(r"async fn api_kyc_status.*?\]\n        \}\n    \}\n\}", "", content, flags=re.DOTALL)
content = re.sub(r"/// POST /api/kyc/submit(.*?)async fn api_kyc_submit.*?\]\n                \.into_response\(\);\n        \}\n    \}\n\}", "", content, flags=re.DOTALL)

with open("src/main.rs", "w", encoding="utf-8") as f:
    f.write(content)

