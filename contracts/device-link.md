# Contract: device link proof

A machine proves to the API (Go) that it holds an Ed25519 key, in order to be attached to a
user account. The same bytes are produced by Rust (`sil-crypto`, `Domain::DeviceLink`) and
checked by Go (`services/api/internal/devices`).

## Bytes signed

```
signed = "sil/v1/device-link" || 0x00 || code || public_key
```

- `code`: the 12 ASCII characters of the one-time link code, **upper case** (the API
  upper-cases what the user typed before checking).
- `public_key`: the 32 raw bytes of the Ed25519 public key.
- Signature: Ed25519 (RFC 8032), 64 bytes.

The domain tag makes a link signature useless for any other purpose (events, attestations,
settlements use other tags), and the code makes it useless for another account or another
moment (one-time, 10 minutes).

## Wire format

`POST /v1/devices/link` (no session; the proof is the authentication):

```json
{ "code": "ABCDEFGHJKLM", "public_key": "<64 lower-case hex>", "signature": "<128 hex>", "label": "my PC" }
```

## Preview and one-click link

`GET /v1/devices/link-codes/{code}` (no session, rate-limited) returns the account the code
belongs to, masked (`a***@example.com` or the first and last characters of a wallet address),
and its expiry. It does not consume the code. The website opens the desktop application with
`silicium://link?code=<code>`; the application shows the preview, and the owner confirms before
the proof is signed and sent.

## Test vector

Shared by `services/api/internal/devices` (Go test `TestSharedVectorStaysStable`) and
`crates/sil-crypto` (Rust test `device_link_proof_from_the_go_service_verifies`). If one side
changes, both tests must change together.

| Field | Value |
|---|---|
| seed (32 bytes) | `07` repeated 32 times |
| code | `ABCDEFGHJKLM` |
| public key | `ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c` |
| signature | `146c5c2c78b2a2bf29904f13fe926374ee5f343034ae2fdc763943c45ed00e714a0b892b48ecab33df2c1ef8174511b9f8a24e7a70d45a10e4c899bbda83ae0b` |

## Known difference

Rust verifies with `verify_strict` (rejects weak keys and malleable signatures); Go's
`ed25519.Verify` does not reject small-order keys. A device registering a weak key only
harms itself (it can sign nothing useful), and the key is never used by the API to verify
ledger data. The ledger itself is always verified by the Rust node.
