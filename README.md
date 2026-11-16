# Silicium v1

Silicium is a verifiable distributed computing network. Volunteer machines run tasks (computation,
embeddings, rendering), other machines verify the results, and payments settle on Solana. A result
is accepted only after verification, and no machine is paid for work it did not do.

This repository is the **meta-repository**: architecture decisions, the release workflow, the
packaging and deployment scripts, and the pinned versions of every component. It is public. The
components are separate repositories, pinned here as git submodules.

## Components

| Path | Repository | Visibility | Content |
|---|---|---|---|
| `network/` | `v1-network` | private (public later) | P2P protocol, signed ledger, node, workloads (Rust workspace) |
| `apps/desktop/` | `v1-desktop` | private | Tauri desktop app: installs and supervises the node (Windows, Linux) |
| `apps/web/` | `v1-web` | private | Public website, network status, downloads, job pages (Vue, Vite) |
| `services/api/` | `v1-api` | private | Accounts, sessions, gateway to the node (Go, SQLite) |
| `chain/` | `v1-chain` | private | Solana program, chain adapter, devnet tooling (one Cargo workspace per crate) |
| `infra/` | this repository | public | Packaging script, VPS deploy and backup scripts, systemd units |
| `tools/` | this repository | public | Runtime fetch script (ONNX Runtime) |
| `docs/architecture/` | this repository | public | Architecture decision records (ADR) |

`components.lock.json` lists the exact commit of each component. A component change reaches dev or
production only when its pin is updated in this repository.

## Clone

```bash
git clone --recurse-submodules https://github.com/Silicium-Project/v1.git
```

Private submodules need read access to the component repositories. Each component builds and tests
on its own; see its README.

## Branches and environments

- `main` is production. A push to `main` runs the release workflow in the `production` environment.
- `dev` is integration. A push to `dev` runs the same workflow in the `development` environment.
- Feature branches start from `dev` and come back by pull request.

| Environment | Site | Node and API |
|---|---|---|
| production | https://siliciumlabs.org | `/opt/silicium`; services `silicium-node`, `silicium-api` |
| development | https://10.77.0.1:8443, WireGuard VPN only (`infra/vpn/`) | `/opt/silicium-dev`; services `silicium-node-dev`, `silicium-api-dev` |

Both environments run on the same VPS, with separate paths, services, data directories, ports and
node identities. The development node does not bootstrap from production.

## Release workflow

`.github/workflows/release.yml` runs on each push to `dev` or `main`:

1. **tests**: format, clippy, `cargo test`, `go test`, `npm test`, and `cargo check` for each chain
   crate, on the pinned commits. Blocking: no later job runs if a suite fails.
2. **linux-binaries** and **windows-binaries**: node, raytracer, sandbox launcher, ONNX Runtime, API,
   WebAssembly render engine.
3. **linux-packages**: AppImage, `.deb` and `.rpm` (`infra/release/build-linux-desktop.sh`).
4. **windows-installer**: NSIS installer, signed with the code-signing certificate stored as an
   environment secret (`SIGN_PFX_B64`, `SIGN_PFX_PASSWORD`).
5. **site**: website build, with node bundles, installers and checksums in `downloads/`.
6. **deploy**: `infra/vps/deploy.sh` with `SIL_ENV=prod` (on `main`) or `SIL_ENV=dev` (on `dev`), then
   the site copied to the matching directory.

CI does not make backups. Run `infra/vps/backup.sh` from a workstation before approving a production
deployment. Backups are encrypted to a GPG public key; the private key is kept outside the workstation.

**Not yet enabled:** required reviewers on the `production` environment, and branch protection on
`main`. Until both are enabled, a push to `main` deploys to production without approval.

### Versions

The version appears in: `apps/desktop/package.json`, `apps/desktop/src-tauri/Cargo.toml`,
`apps/desktop/src-tauri/tauri.conf.json`, `apps/web/scripts/copy-app-installer.mjs` (`VERSION`), and
`infra/systemd/silicium-node.service` (`SIL_APP_VERSION`). Update all of them in one commit.

## Package repositories (DNF and APT)

The website install page tells users to add `https://siliciumlabs.org/download/silicium.repo` and
run `dnf install silicium`. The current state:

- The DNF repository (`/download/dnf/`) and APT repository (`/download/apt/`) are **built by hand** on
  the VPS and are **out of date**. The DNF repository holds packages from 0.1.2 to 0.1.7. Version 1.9.0
  is not in it.
- The DNF repository metadata is signed (`repomd.xml.asc`), and `silicium.repo` sets `repo_gpgcheck=1`.
- Nothing in the repository generates it. The planned process: CI copies the new `.rpm` and `.deb`
  files, runs `createrepo_c` and the APT indexing tool, signs the metadata with the release key, and
  publishes the result under `downloads/`. This is not implemented yet.

Until then, the install page points users to an outdated repository.

## Language

All documentation is in English: READMEs, ADRs, design comments and commit messages. The ADRs in
`docs/architecture/` (0001 to 0014) are currently in French and need translation.

## Linting

No linter configuration is committed. CI enforces `rustfmt`, `clippy -D warnings`, `gofmt`, `go vet`,
TypeScript type checking and the test suites. No Markdown linter is set up.

## Security

Report vulnerabilities to the maintainers in private. Do not open a public issue.
Never commit a key, a seed (`node.seed`), a token, a signing certificate or an `.env` file.

## License

MIT, as declared in the Rust workspace manifests.
