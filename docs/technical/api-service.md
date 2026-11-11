# Service API (Go) — `services/api`

The API is the **account and job front door**. It sits between users (web, desktop) and a
node's control API. It never touches the P2P network, never holds a user's signing key, and
never decides what is true about a job: the ledger does.

```
web / desktop ──HTTP/JSON──▶ services/api ──HTTP (token)──▶ sil-control-api (node) ──▶ P2P
```

## Layout (one folder per concern, files ≤ 300 lines)

| Folder | Role |
|---|---|
| `cmd/silicium-api` | `main`: load config, wire, serve, graceful stop. No logic. |
| `internal/config` | the only reader of `SIL_API_*` variables; reports all problems at once |
| `internal/store` | accounts, sessions, link codes, devices, job index (SQLite, `modernc.org/sqlite`, sans cgo) |
| `internal/auth` | register, login, sessions (PBKDF2-SHA256, hashed tokens) |
| `internal/devices` | one-time link codes and the Ed25519 proof (`contracts/device-link.md`) |
| `internal/jobs` | upload input, forward signed events, honest status |
| `internal/nodeclient` | HTTP client of the node control API |
| `internal/httpx` | JSON helpers, body limits, rate limiter, middleware |
| `internal/server` | routes and handlers only |

Standard library only (no third-party module): fewer supply-chain risks, nothing to audit
besides the Go toolchain.

## Routes

| Route | Auth | Purpose |
|---|---|---|
| `GET /health` | none | liveness |
| `POST /v1/auth/register` | none, rate-limited | create an account (can be closed) |
| `POST /v1/auth/login` | none, rate-limited | returns a bearer token |
| `POST /v1/auth/logout` | session | ends the session |
| `POST /v1/devices/link-codes` | session | one-time code, 10 minutes |
| `POST /v1/devices/link` | proof of key | the machine links itself |
| `GET /v1/devices/link-codes/{code}` | none, rate-limited | which account a code belongs to (masked), before the owner confirms; the code is not consumed |
| `GET /v1/devices`, `DELETE /v1/devices/{id}` | session | list, revoke |
| `PUT /v1/content` | session | upload an input (raw bytes) to the node |
| `POST /v1/jobs` | session | forward a **client-signed** `JobPublished` event |
| `GET /v1/jobs`, `GET /v1/jobs/{id}` | session | my jobs, and one job with its status |
| `POST /v1/auth/wallet/challenge` | none, rate-limited | single-use message for a Solana wallet to sign (login) |
| `POST /v1/auth/wallet/login` | none, rate-limited | verifies the Ed25519 signature, opens a session; creates the account on first login |
| `POST /v1/account/delete-challenge` | session | message to sign before deleting a wallet account |
| `GET /v1/account` | session | the account (e-mail, wallet address, creation date) |
| `GET /v1/account/export` | session | every personal datum held, as a JSON download (GDPR art. 15, 20) |
| `DELETE /v1/account` | session + password, or session + fresh delete signature | erases the account and its sessions, link codes, devices and job index (GDPR art. 17) |

## Wallet login

A Solana address is an Ed25519 public key, so the wallet's signature is checked directly with
`crypto/ed25519`. The signed text is `Silicium login
Address: <base58>
Nonce: <hex>`; the API
builds it itself from the stored challenge, so the client cannot choose what is signed. A nonce is
single-use, bound to the address and to the purpose (`login` or `delete`), and valid 5 minutes. A
failed attempt consumes it. Challenges live in memory: a restart invalidates them, which only
means asking again. The limit is 10 000 open challenges (`503 busy` beyond).

## Network terms

New jobs get no committee (ADR 0007): the requester signs the terms the network sets.
## Who signs what

The API **does not sign** jobs. The requester signs the `JobSpec` and the `JobPublished`
event with their own key (desktop app or wallet), as the node expects
(`docs/technical/security/control-api.md`). The API receives the signed event (base64 of the
canonical CBOR), forwards it, then checks that the node now knows the job before indexing it.
Building and signing the CBOR event is the client's job (Phase 8 desktop/web); the Go side
deliberately has no CBOR encoder, so it cannot produce a bad signed payload.

## Honest job status

`GET /v1/jobs/{id}` returns `state` as one of:

| State | Meaning |
|---|---|
| `running` | the node reports the job active |
| `finalized` | finalized **in the ledger**; payment on chain is separate and not implied |
| `cancelled`, `expired` | as reported by the node |
| `not_on_node` | the node does not know the job (yet, or any more) |
| `unknown` | the node could not be asked, or answered something unreadable |

V0 could show a job as done or failed when the truth was "we could not ask". Here a node
outage is always `unknown`, a publish during an outage is `503 node_unavailable` and records
nothing (nothing is indexed that the node did not confirm), and a status the API does not
recognise is `unknown`, never a guess.

## Security decisions

| Topic | Decision |
|---|---|
| Passwords | PBKDF2-HMAC-SHA256, 600 000 iterations, 16-byte salt, 12–128 characters; unknown e-mails cost the same time as wrong passwords; same error for both |
| Sessions | 256-bit random bearer tokens, only their SHA-256 stored, 24 h default; **bearer header, no cookie**, so no CSRF surface |
| Brute force | per-address limit on register/login/link (1/s, burst 10), per-account limit on login (5 attempts, then 1 per 3 min), global limit per address |
| Device link | one-time code (60 bits, no ambiguous characters), 10 min, hashed at rest; proof is verified **before** the code is consumed and the code is consumed atomically with the device insert |
| Forwarded headers | ignored for rate limiting (forgeable); put a proxy that rewrites the peer address if needed |
| Bodies | JSON 128 KiB, content 64 MiB, unknown fields and trailing data refused |
| Errors | one JSON shape; unexpected errors are logged, never returned |
| Logs | method, path, status, duration, request id; never headers, queries or bodies |
| Node token | read from the node token file (`read,submit` line), never logged, redacted from client errors |
| Authorization | a user only sees and reads their own jobs and devices (`404`, not `403`, for others') |

## Web and desktop clients

The web app is served by the API itself (`SIL_API_WEB_DIR`, same origin, strict CSP). The desktop
shell calls the API from its own origin: those origins must be listed in `SIL_API_CORS_ORIGINS`.
CORS is exact-match only, never with credentials. See `web-and-desktop.md`.

## Limits (honest)

- **No TLS in the service.** Run it behind a TLS-terminating proxy; the session token and
  password travel in clear otherwise.
- **SQLite store**: one instance only. The data is loaded in memory at start; each change is
  written in one transaction that touches only the rows that changed, and is rolled back in memory
  if the write fails. Fine for thousands of accounts. Not for several API instances: that needs a
  server database.
- **Job ownership is bookkeeping**: the index says "this account submitted it"; another
  account could register a job id that already exists on the node by sending a valid known
  event first. It grants nothing (authority is the requester signature in the ledger) but
  "my jobs" is not proof of authorship.
- **No e-mail verification, password reset or 2FA.**
- **Rate limiter is per process** and keyed by TCP peer address.
- Settlement/payment status is not exposed per job yet: the node lists settlements by intent
  id, not by job. Needs a node-side link first.
- Go's Ed25519 verification is not "strict" (see `contracts/device-link.md`).
