# Authentication & Session Flow

This document details the secure authentication mechanism used by the POOOL platform.

```mermaid
sequenceDiagram
    participant U as User (Browser)
    participant F as Frontend (HTMX/Alpine)
    participant B as Rust Backend (Axum)
    participant D as PostgreSQL
    
    Note over U,D: Login Process
    U->>F: Enter Credentials
    F->>B: POST /auth/login (email, password)
    B->>D: SELECT user_data WHERE email = ?
    D-->>B: User Record (incl. Argon2id Hash)
    
    Note right of B: Verify Password (Argon2id)
    
    alt Success
        B->>D: INSERT INTO user_sessions (user_id, token, expires_at)
        D-->>B: Success
        B-->>U: Set-Cookie: session_id=... (HTTPOnly, Secure, SameSite=Lax)
        B-->>F: HTTP 303 Redirect /marketplace
    else Failure
        B-->>F: HTTP 401 Unauthorized
        F-->>U: Show Error (Alpine.js Toast)
    end

    Note over U,D: Protected Request (HTMX)
    U->>F: Click "Portfolio"
    F->>B: GET /portfolio (Browser attaches Cookie)
    B->>D: SELECT user FROM sessions WHERE token = ? AND expires > NOW()
    D-->>B: User Profile
    B-->>F: Return HTML Partial (HTMX Fragment)
    F-->>U: Swap Partial into DOM
```

## Security Features
1. **Password Hashing**: Argon2id is used for all password storage, providing resistance against GPU/ASIC cracking.
2. **Session Security**:
   - `HttpOnly`: Prevents Cross-Site Scripting (XSS) from stealing session tokens.
   - `Secure`: Ensures cookies are only sent over HTTPS.
   - `SameSite=Lax`: Protects against Cross-Site Request Forgery (CSRF).
3. **Session Revocation**: Admins can immediately invalidate all active sessions for a specific user ID.
