# services/api

Go service: accounts, sessions, machine linking, job index and honest job status, in front of
a Silicium node. Standard library only.

```bash
go test ./...
```

Run (the node must be running with its control API):

```bash
SIL_API_NODE_URL=http://127.0.0.1:8750 SIL_API_NODE_TOKEN_FILE=../../data/control.token go run ./cmd/silicium-api
```

Configuration (all optional except the node):

| Variable | Default | Meaning |
|---|---|---|
| `SIL_API_NODE_URL` | required | control API of the node |
| `SIL_API_NODE_TOKEN_FILE` | required | token file with a `read,submit` line |
| `SIL_API_ADDR` | `127.0.0.1:8080` | listen address (no TLS: use a proxy) |
| `SIL_API_STORE` | `data/store.json` | JSON store file |
| `SIL_API_ALLOW_REGISTRATION` | `true` | `false` closes sign-up |
| `SIL_API_SESSION_HOURS` | `24` | session lifetime, 1 to 720 |
| `SIL_API_LOG_JSON` | `false` | JSON logs |
| `SIL_API_WEB_DIR` | empty | built web app (`apps/web/dist`) served at `/`, same origin |
| `SIL_API_COMMITTEE` | empty | coordinator keys given to new jobs (64 lower-case hex, 1 to 7, comma-separated); required to submit |
| `SIL_API_CORS_ORIGINS` | empty | comma-separated origins allowed to call the API, e.g. `http://tauri.localhost,tauri://localhost` for the desktop shell |

Technical doc: `../../docs/technical/api-service.md`. User doc: `../../docs/user/use-the-api.md`.
Contract with Rust: `../../contracts/device-link.md`.
