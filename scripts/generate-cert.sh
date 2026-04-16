#!/bin/bash
set -euo pipefail

# Generate a self-signed TLS certificate for Traefik.
# Output: ./certs/server.crt and ./certs/server.key (relative to CWD).
# Intended to be run from the directory containing docker-compose.yaml.

# Preflight: openssl is required.
if ! command -v openssl >/dev/null 2>&1; then
  echo "❌ openssl is not installed. Install it with your package manager and re-run."
  echo "   Debian/Ubuntu: sudo apt-get install -y openssl"
  echo "   RHEL/Fedora:   sudo dnf install -y openssl"
  exit 1
fi

# Preflight: hostname -I is required for IP auto-detection.
if ! command -v hostname >/dev/null 2>&1; then
  echo "❌ 'hostname' command is not available. This script targets Linux."
  exit 1
fi

IP_ADDR="${1:-$(hostname -I | awk '{print $1}')}"
if [[ -z "${IP_ADDR}" ]]; then
  echo "❌ Unable to determine host IP. Pass it explicitly: $0 <ip>"
  exit 1
fi

CERT_PATH="./certs/server.crt"
KEY_PATH="./certs/server.key"

mkdir -p certs

if [[ -s "$CERT_PATH" && -s "$KEY_PATH" ]]; then
  echo "✅ SSL cert already exists in ./certs/, skipping generation."
  exit 0
fi

echo "🔐 Generating self-signed SSL cert in ./certs/ for $IP_ADDR"
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$KEY_PATH" \
  -out "$CERT_PATH" \
  -days 365 \
  -subj "/CN=$IP_ADDR" \
  -addext "subjectAltName=IP:$IP_ADDR"

echo "✅ Certificate generated: $CERT_PATH"
