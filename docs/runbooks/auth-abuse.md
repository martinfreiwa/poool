# Runbook: AuthEndpointAbusePattern

## Alert source

`infra/prometheus/alerts.yml` → `AuthEndpointAbusePattern`

Fires (page severity) when `/auth/login`, `/auth/2fa`, or
`/auth/forgot-password` returns 4xx at >1 req/sec sustained for 15 min.

## What this means

Almost always one of three things, in rough probability order:

1. **Credential-stuffing attack** — botnet trying lists of leaked
   email/password pairs. Visible as 401s on `/auth/login` from many IPs.
2. **OTP brute-force** — attacker has valid credentials, hammering
   `/auth/2fa` to guess the 6-digit TOTP. Visible as 401/400 spikes on
   `/auth/2fa` from a small IP set against ONE user.
3. **Password-reset spam** — a script enumerating `/auth/forgot-password`
   to either (a) confirm which emails are registered (account-existence
   oracle) or (b) annoy users with reset emails.

The dual-tier rate limiter (IP bucket + email/user bucket) is the
defence — this alert shows it's being exercised. Page severity because
a sustained pattern means our defence is being PROBED, not just
incidentally tripped.

## Triage in 10 minutes

1. Open the Grafana **POOOL Backend HTTP** dashboard → filter
   `path =~ "/auth/.*"` and `status_class = "4xx"`. Identify which
   endpoint is hot.
2. Tail the structured logs for the suspicious endpoint:
   ```
   grep '/auth/login\|/auth/2fa\|/auth/forgot-password' /var/log/backend.log \
     | grep -E 'status=4' \
     | tail -200
   ```
   Group by `remote_addr` and `email` (where logged):
   - Many IPs, many emails, one IP per email → distributed
     credential stuffing.
   - Many IPs against ONE email → targeted attack on that user;
     freeze the account.
   - Few IPs against many emails → a single attacker source, easy
     to block.

3. Cross-reference against `compliance_alerts` — if any of the
   targeted emails belong to a flagged user, escalate to compliance.

## Immediate mitigations

| Pattern | Action |
|---|---|
| Single source IP / small IP set | Block at the edge (Cloudflare WAF) before they exhaust the rate-limit bucket repeatedly. |
| Targeted OTP brute-force on one user | Freeze the user account (admin → users → `user_status='frozen'`); email them about a "security event" and require password+2FA reset. |
| Credential stuffing from a botnet | Tighten Cloudflare bot-mode to "Under Attack"; enable JS challenge on /auth/*; verify our breach-password check (`backend/src/auth/password.rs`) is rejecting the common 10k. |
| Reset-email spam | Add rate-limit on `/auth/forgot-password` per *email* (not just IP). Currently the IP-only bucket lets attackers cycle through emails freely. |

## Don't

- Don't unblock IPs without checking them in the breach-IP list first.
- Don't email "your account is being attacked" to the user *during*
  the incident — wait until you've forced the password+2FA reset, or
  attackers can confirm hits.
- Don't post the specific email patterns in `#general` — keep
  attacker tactics to security channels only.

## Related findings

See `docs/security-audits/2026-05-16-admin-auth-followup.md` finding
M#9: the dual `RateLimiter::check()` calls are non-atomic. Until that
finding lands, an attacker bursting between the two checks can bypass
the second tier on parallel requests. If this alert fires while M#9 is
still open, treat the rate-limit numbers as a soft ceiling.
