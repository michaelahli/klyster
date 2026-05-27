#!/bin/bash
# Build Klyster Docker images (core + seer sidecar)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE_NAME="${IMAGE_NAME:-klyster}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
SEER_IMAGE_NAME="${SEER_IMAGE_NAME:-klyster-seer}"
SEER_IMAGE_TAG="${SEER_IMAGE_TAG:-latest}"

TARGET="${1:-all}"

cd "$PROJECT_ROOT"

build_core() {
    echo "Building Klyster core image: ${IMAGE_NAME}:${IMAGE_TAG}"
    docker build \
        -f build/Dockerfile \
        -t "${IMAGE_NAME}:${IMAGE_TAG}" \
        .
    echo "✅ Core build complete: ${IMAGE_NAME}:${IMAGE_TAG}"
}

build_seer() {
    echo "Building Klyster seer image: ${SEER_IMAGE_NAME}:${SEER_IMAGE_TAG}"
    docker build \
        -f build/Dockerfile.seer \
        -t "${SEER_IMAGE_NAME}:${SEER_IMAGE_TAG}" \
        .
    echo "✅ Seer build complete: ${SEER_IMAGE_NAME}:${SEER_IMAGE_TAG}"
}

case "$TARGET" in
    core)
        build_core
        ;;
    seer)
        build_seer
        ;;
    all)
        build_core
        build_seer
        ;;
    *)
        echo "Usage: $0 [core|seer|all]" >&2
        exit 1
        ;;
esac

echo ""
echo "Run with docker-compose:"
echo "  cd build && docker-compose up"
