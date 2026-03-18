use reqwest::Client;
use serde_json::json;

/// Send an email using Resend API.
pub async fn send_email(to: &str, subject: &str, html_body: &str) -> Result<(), crate::error::AppError> {
    let api_key = match std::env::var("RESEND_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => {
            tracing::warn!("RESEND_API_KEY not configured — email to {} not sent", to);
            return Ok(()); // Silently skip in dev; in prod this env var must be set
        }
    };
    
    let client = Client::new();
    let res = client.post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&json!({
            "from": "POOOL <hello@poool.app>",
            "to": [to],
            "subject": subject,
            "html": html_body
        }))
        .send()
        .await
        .map_err(|e| crate::error::AppError::Internal(format!("Failed to send email request: {}", e)))?;
        
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        tracing::error!("Resend API error ({}): {}", status, text);
        return Err(crate::error::AppError::Internal("Failed to send email via Resend".into()));
    }

    Ok(())
}
