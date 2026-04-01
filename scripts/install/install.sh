#!/usr/bin/env bash
set -euo pipefail

REPO="NexusQuantum/installer-NQRust-Portal"
BIN="nqrust-portal"

ARCH_RAW="$(uname -m)"
case "${ARCH_RAW}" in
  x86_64|amd64)
    ARCH="amd64"
    ;;
  aarch64|arm64)
    echo "[ERROR] arm64 builds are not published yet. Please use an x86_64 machine for now." >&2
    exit 1
    ;;
  *)
    echo "[ERROR] Unsupported architecture: ${ARCH_RAW}" >&2
    exit 1
    ;;
esac

BASE="https://github.com/${REPO}/releases/latest/download"
DEB_NAME="${BIN}_${ARCH}.deb"
DEB_URL="${BASE}/${DEB_NAME}"
SUMS_URL="${BASE}/SHA256SUMS"
TMPDIR="$(mktemp -d)"
cleanup() { rm -rf "${TMPDIR}"; }
trap cleanup EXIT

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "[ERROR] Missing required command: $1" >&2
    exit 1
  }
}

need_cmd curl
need_cmd sha256sum
need_cmd sudo
need_cmd python3

fetch() {
  local url="$1" out="$2"
  echo "[INFO] Downloading ${url}"
  curl -fL --retry 3 --retry-delay 1 -o "${out}" "${url}"
}

if ! fetch "${DEB_URL}" "${TMPDIR}/${DEB_NAME}"; then
  echo "[WARN] Alias ${DEB_NAME} not found, falling back to latest versioned .deb" >&2
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
  ASSET_URL=$(curl -fsSL "${API_URL}" | python3 -c "import sys, json; data=json.load(sys.stdin); assets=data.get('assets',[]); deb=[a['browser_download_url'] for a in assets if a['name'].endswith('_${ARCH}.deb')]; print(deb[0] if deb else '')")
  if [[ -z "${ASSET_URL}" ]]; then
    echo "[ERROR] Could not locate a .deb asset for arch ${ARCH} in the latest release." >&2
    exit 1
  fi
  DEB_NAME=$(basename "${ASSET_URL}")
  fetch "${ASSET_URL}" "${TMPDIR}/${DEB_NAME}"
fi

fetch "${SUMS_URL}" "${TMPDIR}/SHA256SUMS"

echo "[INFO] Verifying checksum"
if ! (cd "${TMPDIR}" && grep "${DEB_NAME}" SHA256SUMS | sha256sum -c -); then
  echo "[ERROR] Checksum verification failed" >&2
  exit 1
fi

echo "[INFO] Installing package"
if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update -y
  sudo apt-get install -y "${TMPDIR}/${DEB_NAME}"
else
  sudo dpkg -i "${TMPDIR}/${DEB_NAME}"
  sudo apt-get install -f -y || true
fi

echo "[INFO] Installed. Run: ${BIN}"
