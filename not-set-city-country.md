# "(not set)" City / Country — Root Cause

## The Problem

Analytics / geo-IP lookups show **"(not set)"** for city and country because
the raw `x-forwarded-for` header value is stored as-is into `user_sessions.ip_address`.

`x-forwarded-for` is a **comma-separated proxy chain**, e.g.:

```
X-Forwarded-For: 1.2.3.4, 10.0.0.1, 172.16.0.2
                 ^^^^^^^^^^  real client IP
                             ^^^^^^^^^^^^^^^^^^^^^^  proxy hops
```

Any geo-IP resolver given the full string `"1.2.3.4, 10.0.0.1, 172.16.0.2"`
fails to match a database entry → returns null → analytics shows **(not set)**.

---

## Affected Code

**File:** `backend/src/auth/routes.rs`

### 1. Login handler (~line 189) — session creation

```rust
// ❌ stores the full proxy chain, not just the client IP
let ip = headers
    .get("x-forwarded-for")
    .or_else(|| headers.get("x-real-ip"))
    .and_then(|v| v.to_str().ok())
    .map(|s| s.to_string());          // e.g. "1.2.3.4, 10.0.0.1"
```

### 2. Signup handler (~line 478) — session + consent creation

```rust
// ❌ same issue
let ip = headers
    .get("x-forwarded-for")
    .or_else(|| headers.get("x-real-ip"))
    .and_then(|v| v.to_str().ok())
    .map(|s| s.to_string());          // e.g. "1.2.3.4, 10.0.0.1"
```

---

## Contrast — Rate-limit IP extraction (already correct)

The rate-limit blocks (lines 154–163 and 434–443) **do** split correctly:

```rust
// ✅ correctly extracts only the first (real client) IP
let client_ip = headers
    .get("x-forwarded-for")
    .or_else(|| headers.get("x-real-ip"))
    .and_then(|v| v.to_str().ok())
    .unwrap_or("unknown")
    .split(',')       // ← split on commas
    .next()           // ← take first = real client
    .unwrap_or("unknown")
    .trim()
    .to_string();
```

---

## Fix

Apply the same `.split(',').next().trim()` pattern to both session `ip` variables:

```rust
// ✅ fixed version (apply to both login and signup handlers)
let ip = headers
    .get("x-forwarded-for")
    .or_else(|| headers.get("x-real-ip"))
    .and_then(|v| v.to_str().ok())
    .map(|s| s.split(',').next().unwrap_or(s).trim().to_string());
```

This ensures `user_sessions.ip_address` always holds a single, clean IP
that any geo-IP service can resolve to city + country.
