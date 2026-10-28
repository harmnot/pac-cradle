# Paddle pre-production runbook (RepVane backend)

This runbook documents the exact CLI flow to prepare Paddle for RepVane before production.

`PADDLE_WEBHOOK_SECRET` in this repo must be the webhook destination secret key (`pdl_ntfset_...`) for:

`https://<your-api-host>/api/v1/webhooks/paddle`

That value is used by the backend to verify `Paddle-Signature` on `/api/v1/webhooks/paddle`.

## 1) Prepare API keys

Create two API keys in Paddle Dashboard:

- **Sandbox key** (`pdl_sk_sdbx_...`) for development/testing.
- **Live key** (`pdl_sk_live_...`) for production.

Use least privilege, but include webhook and billing operations needed by this backend (`notification_setting.*`, `transaction.*`, `subscription.*`, `adjustment.*`).

## 2) Login and verify environment

```bash
# Sandbox login
paddle login --environment sandbox --api-key "pdl_sk_sdbx_xxx"
paddle whoami

# Production login (when ready for live)
paddle login --environment production --api-key "pdl_sk_live_xxx"
paddle whoami
```

You can switch quickly:

```bash
paddle sandbox
paddle production
```

## 3) Create webhook destination and get `PADDLE_WEBHOOK_SECRET`

### 3.1 Sandbox destination

Create request body file:

```bash
cat > /tmp/repvane-paddle-webhook-sandbox.json <<'JSON'
{
  "description": "RepVane API Sandbox Webhook",
  "type": "url",
  "destination": "https://<sandbox-api-host>/api/v1/webhooks/paddle",
  "traffic_source": "all",
  "subscribed_events": [
    "transaction.completed",
    "subscription.activated",
    "subscription.created",
    "subscription.updated",
    "subscription.trialing",
    "subscription.past_due",
    "subscription.paused",
    "subscription.resumed",
    "subscription.canceled",
    "adjustment.created",
    "adjustment.updated"
  ]
}
JSON
```

Create destination:

```bash
paddle notification-settings create \
  --environment sandbox \
  --json @/tmp/repvane-paddle-webhook-sandbox.json \
  -o json
```

From that JSON response, copy:

- `data.id` (notification destination id, `ntfset_...`)
- `data.endpoint_secret_key` (webhook secret, `pdl_ntfset_...`) → this is your `PADDLE_WEBHOOK_SECRET`

> **Important:** keep the created destination ID (`ntfset_...`) and `endpoint_secret_key` (`pdl_ntfset_...`) in your secrets manager.

### 3.2 Production destination

Repeat with production URL:

```bash
cat > /tmp/repvane-paddle-webhook-production.json <<'JSON'
{
  "description": "RepVane API Production Webhook",
  "type": "url",
  "destination": "https://api.repvane.com/api/v1/webhooks/paddle",
  "traffic_source": "all",
  "subscribed_events": [
    "transaction.completed",
    "subscription.activated",
    "subscription.created",
    "subscription.updated",
    "subscription.trialing",
    "subscription.past_due",
    "subscription.paused",
    "subscription.resumed",
    "subscription.canceled",
    "adjustment.created",
    "adjustment.updated"
  ]
}
JSON

paddle notification-settings create \
  --environment production \
  --json @/tmp/repvane-paddle-webhook-production.json \
  -o json
```

To inspect existing destinations:

```bash
paddle notification-settings list --environment sandbox -o json
paddle notification-settings list --environment production -o json
```

## 4) Set backend `.env`

For **sandbox** development:

```dotenv
PADDLE_ENVIRONMENT=sandbox
PADDLE_API_KEY=pdl_sk_sdbx_xxx
PADDLE_WEBHOOK_SECRET=pdl_ntfset_xxx
```

For **production**:

```dotenv
PADDLE_ENVIRONMENT=production
PADDLE_API_KEY=pdl_sk_live_xxx
PADDLE_WEBHOOK_SECRET=pdl_ntfset_xxx
```

Values map to:

- `PADDLE_ENVIRONMENT` → Paddle API base URL (`sandbox` or `production`)
- `PADDLE_API_KEY` → server-side Paddle API key
- `PADDLE_WEBHOOK_SECRET` → webhook destination `endpoint_secret_key`

## 5) Set Paddle default checkout URL (required)

RepVane creates transactions and expects Paddle to generate a checkout URL. If this is missing, API calls can fail with:

- `PAYMENT_PROVIDER_FAILED`
- Paddle code: `transaction_default_checkout_url_not_set`

Set the account-level default checkout URL:

```bash
# sandbox
paddle settings checkout-url update \
  --environment sandbox \
  --json '{"default_url":"https://<your-app-host>/pay"}' \
  -o json

# production
paddle settings checkout-url update \
  --environment production \
  --json '{"default_url":"https://app.repvane.com/pay"}' \
  -o json
```

Verify:

```bash
paddle settings checkout-url get --environment sandbox -o json
paddle settings checkout-url get --environment production -o json
```

Notes:

- `default_url` **must be HTTPS**.
- Domain must be approved in Paddle checkout domains.
- If CLI returns `forbidden`, your API key lacks permissions to read/write checkout URL settings; create/use a key with sufficient settings permissions.

## 6) Post-change checks

After updating `.env` and restarting backend:

1. Ensure API route is reachable: `POST /api/v1/webhooks/paddle`
2. Send a Paddle simulation or test event to confirm signature verification passes.
3. Confirm webhook rows are stored once (deduplicated by provider event id).

## 7) Manual webhook replay with `curl` (local debugging)

Use this when you already have a full Paddle event JSON and want to test backend webhook handling locally.

> Send to `POST /api/v1/webhooks/paddle`, **not** to `/pay?_ptxn=...` (that URL is frontend checkout page only).

```bash
# 1) Save the exact event payload (raw JSON from Paddle)
cat > /tmp/paddle-event.json <<'JSON'
{
  "data": { "...": "paste full payload here" },
  "event_id": "evt_xxx",
  "event_type": "transaction.completed",
  "occurred_at": "2026-10-24T04:25:29.799856Z",
  "notification_id": "ntf_xxx"
}
JSON

# 2) Use the same webhook secret as backend .env
export PADDLE_WEBHOOK_SECRET='pdl_ntfset_xxx'

# 3) Build Paddle-Signature: HMAC_SHA256("<ts>:<raw_body>")
TS=$(date +%s)
SIG=$(printf '%s:%s' "$TS" "$(cat /tmp/paddle-event.json)" \
  | openssl dgst -sha256 -hmac "$PADDLE_WEBHOOK_SECRET" -hex \
  | awk '{print $NF}')

# 4) POST to backend webhook endpoint
curl -i -X POST 'http://localhost:8080/api/v1/webhooks/paddle' \
  -H 'Content-Type: application/json' \
  -H "Paddle-Signature: ts=${TS};h1=${SIG}" \
  --data-binary @/tmp/paddle-event.json
```

Expected success response:

```json
{ "status": "accepted" }
```

Common failures:

- `401 BILLING_WEBHOOK_SIGNATURE_INVALID` → wrong `PADDLE_WEBHOOK_SECRET`, modified payload, or stale timestamp.
- `400 BILLING_WEBHOOK_INVALID` → invalid JSON shape (missing/invalid `event_id`, `event_type`, or payload structure).

## 8) Rotation procedure

When rotating secret/key:

1. Create a new notification destination (or rotate in dashboard).
2. Update `PADDLE_WEBHOOK_SECRET` in secrets/env.
3. Restart backend.
4. Verify new events are accepted.
5. Remove old destination/key.
