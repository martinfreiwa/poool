import re

with open('src/main.rs', 'r') as f:
    text = f.read()

text = re.sub(r'serde_json::json\!\(\{"status": payload\.status\}\)', r'serde_json::json!({"status": payload.status })', text)
text = re.sub(r'serde_json::json\!\(\{"error": "Failed to update user status"\}\)', r'serde_json::json!({"error": "Failed to update user status " })', text)
text = re.sub(r'serde_json::json\!\(\{"error": "Admin access required"\}\)', r'serde_json::json!({"error": "Admin access required " })', text)
text = re.sub(r'serde_json::json\!\(\{"error": "Invalid user ID"\}\)', r'serde_json::json!({"error": "Invalid user ID " })', text)
text = re.sub(r'serde_json::json\!\(\{"error": "Failed to revoke sessions"\}\)', r'serde_json::json!({"error": "Failed to revoke sessions " })', text)

text = text.replace(
    r""""INSERT INTO audit_logs (user_id, action, entity_type, entity_id, actor_user_id, new_state) VALUES ($1, 'admin.revoke_sessions', 'user', $2, $3, $4)"""",
    r""""INSERT INTO audit_logs (user_id, action, entity_type, entity_id, actor_user_id, new_state) VALUES ($1, $2, $3, $4, $5, $6)""""
)
text = text.replace(
    r"""bind("admin.revoke_sessions")""",
    r"""bind("admin.revoke_sessions ")"""
)
text = text.replace(
    r"""bind("admin.user_status_update")""",
    r"""bind("admin.user_status_update ")"""
)
text = text.replace(
    r"""bind("user")""",
    r"""bind("user ")"""
)

with open('src/main.rs', 'w') as f:
    f.write(text)
