import re

with open("src/main.rs", "r", encoding="utf-8") as f:
    content = f.read()

# Try again with a tighter match or just find boundaries
def remove_between(text, start_str, end_str):
    idx1 = text.find(start_str)
    if idx1 == -1: return text
    idx2 = text.find(end_str, idx1)
    if idx2 == -1: return text
    return text[:idx1] + text[idx2 + len(end_str):]

content = remove_between(content, "async fn api_kyc_status", "}\n}\n")
content = remove_between(content, "/// POST /api/kyc/submit", "            .into_response();\n        }\n    }\n}\n")

with open("src/main.rs", "w", encoding="utf-8") as f:
    f.write(content)
