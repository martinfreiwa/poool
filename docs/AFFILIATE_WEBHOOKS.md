# POOOL Affiliate Webhooks

> Verify every incoming POST. Drop signed-out requests.

## Overview

Webhook subscriptions deliver event JSON via signed POST. Configure them via
`POST /api/affiliate/webhooks` (the secret is shown **once**, never recoverable).
Events fire for affiliate-domain lifecycle changes — `commission_earned`,
`payout_released`, `team_invitation_accepted` — plus future additions.

## Request shape

| Header | Value |
|---|---|
| `Content-Type` | `application/json` |
| `X-Poool-Signature` | `sha256=<hex_hmac>` |
| `X-Poool-Event` | event name (e.g. `commission_earned`) |
| `X-Poool-Subscription-Id` | UUID of the firing subscription |

Body:

```json
{
  "event": "commission_earned",
  "subid": "abcd1234",
  "payout_cents": 12345,
  "affiliate_id": "01HZX…",
  "subscription_id": "01HZX…",
  "delivered_at": "2026-05-16T16:00:00Z"
}
```

## Signature spec

```text
signature = "sha256=" || hex( HMAC-SHA256(secret, raw_request_body_bytes) )
```

`secret` is the value returned at subscription-create time. Treat it like
a password. The HMAC is over the **raw body bytes**, not a re-serialised
form — parse the body only after verification.

## Reference: Node.js (Express)

```js
import crypto from 'node:crypto';
import express from 'express';

const app = express();
const POOOL_SECRET = process.env.POOOL_WEBHOOK_SECRET;

// Raw body so the HMAC matches byte-for-byte.
app.post('/poool-webhook',
  express.raw({ type: 'application/json' }),
  (req, res) => {
    const sig = req.headers['x-poool-signature'] || '';
    const expected = 'sha256=' + crypto
      .createHmac('sha256', POOOL_SECRET)
      .update(req.body)
      .digest('hex');
    if (!crypto.timingSafeEqual(Buffer.from(sig), Buffer.from(expected))) {
      return res.status(401).send('bad signature');
    }
    const event = JSON.parse(req.body.toString('utf8'));
    console.log('verified poool event:', event.event, event);
    res.status(200).send('ok');
  });

app.listen(3000);
```

## Reference: Python (Flask)

```python
import hmac, hashlib, os
from flask import Flask, request

app = Flask(__name__)
POOOL_SECRET = os.environ["POOOL_WEBHOOK_SECRET"].encode()

@app.post("/poool-webhook")
def poool():
    sig = request.headers.get("X-Poool-Signature", "")
    expected = "sha256=" + hmac.new(
        POOOL_SECRET, request.get_data(), hashlib.sha256
    ).hexdigest()
    if not hmac.compare_digest(sig, expected):
        return "bad signature", 401
    payload = request.get_json(force=True)
    app.logger.info("verified poool event: %s", payload)
    return "ok", 200
```

## Reference: Go (net/http)

```go
package main

import (
    "crypto/hmac"
    "crypto/sha256"
    "encoding/hex"
    "io"
    "net/http"
    "os"
)

var secret = []byte(os.Getenv("POOOL_WEBHOOK_SECRET"))

func poool(w http.ResponseWriter, r *http.Request) {
    body, _ := io.ReadAll(r.Body)
    mac := hmac.New(sha256.New, secret)
    mac.Write(body)
    expected := "sha256=" + hex.EncodeToString(mac.Sum(nil))
    if !hmac.Equal([]byte(r.Header.Get("X-Poool-Signature")), []byte(expected)) {
        http.Error(w, "bad signature", http.StatusUnauthorized)
        return
    }
    w.Write([]byte("ok"))
}
```

## Reference: PHP

```php
<?php
$secret = getenv('POOOL_WEBHOOK_SECRET');
$body = file_get_contents('php://input');
$signature = $_SERVER['HTTP_X_POOOL_SIGNATURE'] ?? '';
$expected = 'sha256=' . hash_hmac('sha256', $body, $secret);
if (!hash_equals($expected, $signature)) {
    http_response_code(401);
    exit('bad signature');
}
$event = json_decode($body, true);
http_response_code(200);
```

## Retry contract

- Worker retries every 2^N minutes (capped 64m) up to 8 attempts.
- Anything in the `2xx` range = success.
- `4xx` and `5xx` are retried equally; we don't differentiate (returning
  `200` is the only signal to stop retries).
- `failure_count` on the subscription row increments on every failed
  attempt; the UI flags >5 as "Degraded".
- After 8 attempts the outbox row flips to `failed_giveup`. The
  subscription stays active (other events continue to fire); it's up to
  the operator to investigate via the affiliate-settings UI.

## Testing locally

Use [webhook.site](https://webhook.site) for a one-off endpoint:

```sh
curl -sS http://your-poool-host/api/affiliate/webhooks \
  -H 'Content-Type: application/json' \
  -b 'session=...' \
  -d '{"url":"https://webhook.site/<your-uuid>","event_types":"*"}'
```

The response body contains the `secret` — copy it before reloading the
settings page.

## Security checklist

- [ ] Use `crypto.timingSafeEqual` / `hmac.compare_digest` / `hmac.Equal` to compare signatures.
- [ ] Verify the **raw** body, not a JSON-re-serialised form.
- [ ] Store the secret in a secret-manager (Vault / GCP KMS / AWS Secrets Manager), never in code.
- [ ] Reject requests older than 5 minutes if you implement a timestamp header (future addition).
- [ ] Make your handler idempotent — the worker may retry on transient failures.
