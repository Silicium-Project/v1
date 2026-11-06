# Use the account API

The API gives you an account, links your machines, and lists your jobs. Examples use
`curl` against `http://127.0.0.1:8080`. Put the service behind HTTPS before exposing it:
your password and token travel in clear otherwise.

## Sign in with a Solana wallet

1. `POST /v1/auth/wallet/challenge` with `{"address": "<base58>"}` gives `nonce` and `message`.
2. The wallet signs the UTF-8 bytes of `message` (Ed25519).
3. `POST /v1/auth/wallet/login` with `{"address", "nonce", "signature": "<base64>"}` returns the bearer token.

The first valid signature creates the account. A wallet account has no e-mail and no password.

## Create an account and log in

```bash
curl -s -X POST localhost:8080/v1/auth/register -d '{"email":"you@example.com","password":"at least 12 characters"}'
```

```bash
curl -s -X POST localhost:8080/v1/auth/login -d '{"email":"you@example.com","password":"at least 12 characters"}'
```

The answer contains a `token` (valid 24 hours by default). Send it as
`Authorization: Bearer <token>` on every other call. After 5 wrong passwords on one account
you must wait about 3 minutes per further attempt.

## Link a machine

1. Ask for a code (valid 10 minutes, usable once):

```bash
curl -s -X POST localhost:8080/v1/devices/link-codes -H "Authorization: Bearer $TOKEN"
```

2. On the machine, sign the code with the machine key and send the proof to
   `POST /v1/devices/link` (format in `contracts/device-link.md`). The desktop app does this
   for you.

3. List or revoke machines:

```bash
curl -s localhost:8080/v1/devices -H "Authorization: Bearer $TOKEN"
```

## Submit and follow a job

Jobs are signed on **your** side, never by the API. The desktop app builds and signs the
job; it then calls:

- `PUT /v1/content` with the input file as raw bytes (returns the content id);
- `POST /v1/jobs` with `{ "job_id": "<64 hex>", "event": "<base64 signed event>", "label": "..." }`.

Follow it:

```bash
curl -s localhost:8080/v1/jobs/$JOB_ID -H "Authorization: Bearer $TOKEN"
```

## Reading the status

| `state` | What it means for you |
|---|---|
| `running` | the network is working on it |
| `finalized` | the result is final in the ledger. **This does not say you were charged or that payment happened.** |
| `cancelled`, `expired` | ended without a result |
| `not_on_node` | the node you are connected to does not know the job |
| `unknown` | the API could not reach the node. The job may be fine; try again later |

## Common errors

| HTTP | `code` | Meaning |
|---|---|---|
| 401 | `unauthenticated` | missing, wrong or expired token: log in again |
| 429 | `rate_limited` | slow down; see `Retry-After` |
| 503 | `node_unavailable` | the node did not answer; nothing was recorded, retry |
| 422 | `node_rejected` / `not_a_job` | the node refused the event |
