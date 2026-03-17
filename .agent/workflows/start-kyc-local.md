---
description: Start KYC testing locally with ngrok tunnel and Didit webhook forwarding
---

# Local KYC Testing Setup

This workflow sets up everything needed to test the full Didit KYC flow locally.

## Prerequisites

- Backend `.env` has `DIDIT_API_KEY`, `DIDIT_WORKFLOW_ID`, `DIDIT_WEBHOOK_SECRET` set
- ngrok is installed and authenticated (`ngrok config check`)
- `BASE_URL` in `.env` points to your ngrok domain

## Steps

### 1. Start the backend (Terminal 1)
```bash
cd backend && cargo watch -x run
```

### 2. Start ngrok tunnel (Terminal 2)
If you have ngrok static domain (check `.env` for `BASE_URL`):
```bash
ngrok http 8888 --domain=grover-aftmost-willis.ngrok-free.app
```

If you don't have a static domain (URL will change each restart):
```bash
ngrok http 8888
```
Then update `BASE_URL` in `.env` with the new URL and restart backend.

### 3. Configure Didit Webhook URL
Go to https://business.didit.me → Settings → Webhooks
Set webhook URL to: `{BASE_URL}/api/webhooks/kyc/didit`
Example: `https://grover-aftmost-willis.ngrok-free.app/api/webhooks/kyc/didit`

Make sure the webhook secret in Didit matches `DIDIT_WEBHOOK_SECRET` in `.env`.

### 4. Test the KYC flow
1. Open http://localhost:8888/kyc in your browser
2. Click "Start Verification" — you should be redirected to Didit verification page
3. Complete verification in Didit
4. Didit sends webhook to ngrok → forwarded to localhost:8888
5. Check backend logs for: `Processing KYC webhook: user=..., status=approved`

### 5. Manually trigger a webhook for testing (optional)
Use the admin panel to simulate an approval without completing full verification:
```bash
curl -X POST http://localhost:8888/api/admin/kyc/{kyc_record_id}/approve \
  -H "Cookie: poool_session=YOUR_ADMIN_SESSION"
```

## Quick Check: Is Everything Working?

```bash
# Check Didit provider is active (not manual fallback)
curl http://localhost:8888/api/kyc/provider -H "Cookie: poool_session=SESSION"
# Expected: {"provider":"didit","supports_redirect":true}

# Check ngrok is forwarding correctly
curl https://grover-aftmost-willis.ngrok-free.app/health
# Expected: 200 OK from your backend
```
