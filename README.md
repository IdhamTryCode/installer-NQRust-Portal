# NQRust Identity Portal Installer

An interactive Terminal User Interface (TUI) installer for deploying the NQRust Identity Portal stack using Docker Compose.

## Overview

This installer provides a guided two-phase setup experience for the NQRust Identity Portal platform, which includes:

- **Traefik** - Reverse proxy with automatic HTTPS (self-signed TLS)
- **Identity** - OIDC identity provider (OAuth2/OIDC)
- **Identity DB** - PostgreSQL database for Identity and license data
- **Portal** - NQRust Identity Portal (Next.js) with license gate

### Key Features
- **HTTPS by default** - All services served over HTTPS via Traefik self-signed certificates
- **License gate** - Portal and Identity access protected by license verification
- **Two-phase install** - Phase 1 starts the stack, Phase 2 applies Identity (OAuth) client configuration
- **Airgapped support** - Offline installation with bundled Docker images

## Prerequisites

1. **Docker** (engine + CLI) — [Install Docker](https://docs.docker.com/get-docker/)
2. **Docker Compose v2** — `docker compose` (not legacy `docker-compose`)
3. **GitHub Personal Access Token** (PAT) with `read:packages` scope
   - Required to pull container images from `ghcr.io`
   - [Create a PAT](https://github.com/settings/tokens/new) with `read:packages` permission

## Quick Start

Official install uses the **Debian package** (`.deb`) for **amd64**. Releases publish `nqrust-portal_amd64.deb` (and a versioned name like `nqrust-portal_0.0.1_amd64.deb`); the one-liner and `scripts/install/install.sh` match [installer-NQRust-Analytics](https://github.com/NexusQuantum/installer-NQRust-Analytics) behavior (download, `SHA256SUMS` verify, `apt`/`dpkg` install).

### Option A: One-liner install (preferred)

```bash
curl -fsSL https://raw.githubusercontent.com/NexusQuantum/installer-NQRust-Portal/main/scripts/install/install.sh | bash
```

Installs the latest `.deb` from GitHub Releases and adds `nqrust-portal` to your `PATH` (typically `/usr/bin`). Then run:

```bash
nqrust-portal
```

### Option B: Install from release asset

1. Download the latest `.deb` from the [Releases](https://github.com/NexusQuantum/installer-NQRust-Portal/releases) page. Example:

```bash
curl -LO https://github.com/NexusQuantum/installer-NQRust-Portal/releases/latest/download/nqrust-portal_amd64.deb
curl -LO https://github.com/NexusQuantum/installer-NQRust-Portal/releases/latest/download/SHA256SUMS
```

2. Verify (recommended):

```bash
grep nqrust-portal_amd64.deb SHA256SUMS | sha256sum -c -
```

3. Install the package and run:

```bash
sudo apt install ./nqrust-portal_amd64.deb
# or: sudo dpkg -i nqrust-portal_amd64.deb
nqrust-portal
```

### Option C: Build from source

1. Clone the repository:
```bash
git clone https://github.com/NexusQuantum/installer-NQRust-Portal.git
cd installer-NQRust-Portal
```

2. Authenticate with GitHub Container Registry:
```bash
docker login ghcr.io
# Username: your-github-username
# Password: your-personal-access-token (NOT your GitHub password)
```

3. Run the installer:
```bash
cargo run
```

### Option D: Airgapped / Offline Installation

For environments **without internet access** (airgapped, isolated networks, offline VMs):

**On a machine with internet (build machine):**
```bash
# 1. Clone the repository
git clone https://github.com/NexusQuantum/installer-NQRust-Portal.git
cd installer-NQRust-Portal

# 2. Login to GitHub Container Registry
docker login ghcr.io

# 3. Build airgapped binary (~3-4 GB, includes all Docker images)
./scripts/airgapped/build-single-binary.sh
```

The build produces `nqrust-portal-airgapped-installer-<version>-amd64` (e.g. `nqrust-portal-airgapped-installer-0.0.17-amd64`) plus its `.sha256` checksum file. Only `linux/amd64` (x86_64 — both Intel and AMD CPUs) is currently published. The version is derived from the GitHub release tag in CI, the nearest git tag locally, or `Cargo.toml` as a final fallback. Override explicitly with `VERSION=0.0.17 ./scripts/airgapped/build-single-binary.sh`.

**Transfer to airgapped machine** (via USB/SCP/physical media):
```bash
cp nqrust-portal-airgapped-installer-0.0.17-amd64 /path/to/transfer/
cp nqrust-portal-airgapped-installer-0.0.17-amd64.sha256 /path/to/transfer/
```

**On airgapped machine (no internet needed):**
```bash
# 1. Verify checksum
sha256sum -c nqrust-portal-airgapped-installer-0.0.17-amd64.sha256

# 2. Make executable
chmod +x nqrust-portal-airgapped-installer-0.0.17-amd64

# 3. Run installer (auto-extracts and loads Docker images)
./nqrust-portal-airgapped-installer-0.0.17-amd64
```

Or download the pre-built airgapped binary directly from the [Releases](https://github.com/NexusQuantum/installer-NQRust-Portal/releases) page.

> **CI / releases:** The `Build Airgapped Binary` workflow (`.github/workflows/build-airgapped.yml`) logs in to GHCR using the repository secret **`GHCR_TOKEN`** (a GitHub PAT with `read:packages`). Add it under *Settings → Secrets and variables → Actions* or that job will fail when pulling images; the `.deb` **Release** workflow does not depend on it.

## Installation Guide

### Phase 1 — Initial Setup

The installer will prompt for:

| Field | Description | Default |
|-------|-------------|---------|
| Hostname / IP | Server IP or domain (used for HTTPS URLs) | — |
| Portal Port | HTTPS port for the portal | `8083` |
| Identity Port | HTTPS port for Identity | `8082` |
| Admin Password | Identity admin bootstrap password | — |
| Realm Name | Realm name in Identity admin | `master` |
| Client ID | OAuth client ID for the portal | `nqrust-portal` |
| Client Secret | OAuth client secret (leave blank for Phase 1) | — |

After Phase 1 completes:
- Portal is accessible at `https://<hostname>:8083`
- Identity admin console at `https://<hostname>:8082/admin`
- A license key is required to access the portal

### Phase 2 — Apply Identity client configuration

After configuring the OAuth client in the Identity admin console:

1. Open Identity admin console: `https://<hostname>:8082/admin`
2. Create/configure your realm and client (`nqrust-portal`)
3. Set Valid redirect URIs to the OAuth callback URL required by the portal app (typically under `/api/auth/callback/…` on the portal host)
4. Set all URLs (Root, Home, Web origins, Admin) to `https://<hostname>:8083`
5. Copy the client secret
6. Re-run the installer and select **Phase 2** to apply the new secret and realm

## Post-Installation

1. **Activate license** — Visit `https://<hostname>:8083/license-activation` and enter your license key
2. **Login** — Authenticate via Identity (OIDC)
3. **Access portal** — `https://<hostname>:8083/dashboard`

> **Note:** The browser will show a certificate warning because Traefik uses a self-signed certificate. This is expected for self-hosted deployments. Accept the warning to proceed.

## Architecture

```
installer-NQRust-Portal/
├── src/
│   ├── app/           # Application state and two-phase install logic
│   └── ui/            # TUI screens (home, form, progress, success)
├── traefik/
│   └── dynamic.yml    # Traefik HTTPS routers, forwardAuth, services
├── db/
│   └── init.sql       # PostgreSQL schema (license_activations table)
├── scripts/
│   └── airgapped/     # Scripts for building offline binary
├── docker-compose.yaml
└── env_template       # Template for .env generation
```

**Services:**

| Container | Image | Port (external) |
|-----------|-------|-----------------|
| `nqrust-traefik` | `traefik:v3.4` | 8083 (portal), 8082 (identity), 8081 (dashboard) |
| `nqrust-identity` | `ghcr.io/nexusquantum/nqrust-identity:latest` | via Traefik |
| `nqrust-identity-db` | `postgres:16-alpine` | internal only |
| `nqrust-identity-portal` | `ghcr.io/nexusquantum/nqrust-identity-portal:latest` | via Traefik |

## Troubleshooting

### "unauthorized" when pulling images

```bash
docker login ghcr.io
# Use GitHub PAT with read:packages scope as password
```

### Port conflict

Edit `.env` and change `PORTAL_PORT` or `IDENTITY_PORT`, then restart:
```bash
docker compose up -d
```

### Certificate warning in browser

Expected behavior — Traefik uses a self-signed certificate. Click "Advanced" → "Proceed" in the browser.

### Portal stuck on license activation after clearing cookies

Normal behavior if license is in the database — clear the negative cache by waiting a moment and refreshing, or restart the portal container:
```bash
docker compose restart portal
```

### Build errors

```bash
cargo clean && cargo build --release
```

## Development

```bash
# Debug run
cargo run

# Release build
cargo build --release

# Format
cargo fmt

# Lint
cargo clippy
```

## License

Copyright (c) Idham <idhammultazam7@gmail.com>

This project is licensed under the MIT License — see the [LICENSE](./LICENSE) file for details.

## Support

- GitHub Issues: [NexusQuantum/installer-NQRust-Portal](https://github.com/NexusQuantum/installer-NQRust-Portal/issues)
- Email: idhammultazam7@gmail.com
