#!/usr/bin/env bash
set -euo pipefail

# save-images.sh
# Pull and save all Docker images required for NQRust Identity Portal stack
# This script requires internet connection and Docker

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_DIR="${PROJECT_ROOT}/build"
IMAGES_DIR="${BUILD_DIR}/images"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()  { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

if ! command -v docker &> /dev/null; then
    log_error "Docker is not installed or not in PATH"
    exit 1
fi

if ! docker info &> /dev/null; then
    log_error "Docker daemon is not running"
    exit 1
fi

log_info "Creating build directories..."
mkdir -p "${IMAGES_DIR}"

# Images required for NQRust Identity Portal
declare -a IMAGES=(
    "traefik:v3.4|traefik.tar.gz"
    "postgres:16-alpine|postgres.tar.gz"
    "quay.io/keycloak/keycloak:26.1|keycloak.tar.gz"
    "ghcr.io/idhamtrycode/nqrust-identity-portal:latest|nqrust-identity-portal.tar.gz"
)

TOTAL_IMAGES=${#IMAGES[@]}
CURRENT=0

log_info "Starting to pull and save ${TOTAL_IMAGES} Docker images..."
log_info "Platform: linux/amd64"
echo ""

for IMAGE_ENTRY in "${IMAGES[@]}"; do
    CURRENT=$((CURRENT + 1))
    IFS='|' read -r IMAGE_NAME OUTPUT_FILE <<< "${IMAGE_ENTRY}"
    OUTPUT_PATH="${IMAGES_DIR}/${OUTPUT_FILE}"

    log_info "[${CURRENT}/${TOTAL_IMAGES}] Processing: ${IMAGE_NAME}"

    log_info "  Pulling image..."
    if docker pull --platform linux/amd64 "${IMAGE_NAME}"; then
        log_info "  ✓ Pull successful"
    else
        log_error "  ✗ Failed to pull ${IMAGE_NAME}"
        exit 1
    fi

    log_info "  Saving to ${OUTPUT_FILE}..."
    if docker save "${IMAGE_NAME}" | gzip > "${OUTPUT_PATH}"; then
        SIZE=$(du -h "${OUTPUT_PATH}" | cut -f1)
        log_info "  ✓ Saved successfully (${SIZE})"
    else
        log_error "  ✗ Failed to save ${IMAGE_NAME}"
        exit 1
    fi

    echo ""
done

# Generate manifest
MANIFEST_FILE="${IMAGES_DIR}/manifest.json"
log_info "Generating manifest file..."

cat > "${MANIFEST_FILE}" <<EOF
{
  "version": "1.0",
  "platform": "linux/amd64",
  "created": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "images": [
EOF

for i in "${!IMAGES[@]}"; do
    IFS='|' read -r IMAGE_NAME OUTPUT_FILE <<< "${IMAGES[$i]}"
    OUTPUT_PATH="${IMAGES_DIR}/${OUTPUT_FILE}"

    if command -v sha256sum &> /dev/null; then
        CHECKSUM=$(sha256sum "${OUTPUT_PATH}" | cut -d' ' -f1)
    elif command -v shasum &> /dev/null; then
        CHECKSUM=$(shasum -a 256 "${OUTPUT_PATH}" | cut -d' ' -f1)
    else
        CHECKSUM="unavailable"
    fi

    SIZE=$(stat -f%z "${OUTPUT_PATH}" 2>/dev/null || stat -c%s "${OUTPUT_PATH}")
    COMMA=","
    if [ $i -eq $((${#IMAGES[@]} - 1)) ]; then COMMA=""; fi

    cat >> "${MANIFEST_FILE}" <<EOF
    {
      "name": "${IMAGE_NAME}",
      "file": "${OUTPUT_FILE}",
      "size": ${SIZE},
      "sha256": "${CHECKSUM}"
    }${COMMA}
EOF
done

cat >> "${MANIFEST_FILE}" <<EOF
  ]
}
EOF

log_info "✓ Manifest generated"

# Generate checksums
cd "${IMAGES_DIR}"
if command -v sha256sum &> /dev/null; then
    sha256sum *.tar.gz > SHA256SUMS
elif command -v shasum &> /dev/null; then
    shasum -a 256 *.tar.gz > SHA256SUMS
else
    log_warn "No SHA256 tool found, skipping checksums file"
fi
cd "${PROJECT_ROOT}"

log_info ""
log_info "=========================================="
log_info "Docker images saved successfully!"
log_info "=========================================="
log_info "Location: ${IMAGES_DIR}"
log_info "Total images: ${TOTAL_IMAGES}"
log_info "Total size: $(du -sh "${IMAGES_DIR}" | cut -f1)"
log_info ""
log_info "Next step: Run ./scripts/airgapped/build-payload.sh"
