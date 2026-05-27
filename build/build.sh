#!/bin/bash
# Build Klyster Docker images (core + analytics sidecar)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE_NAME="${IMAGE_NAME:-klyster}"
IMAGE_TAG="${IMAGE_TAG:-latest}"
ANALYTICS_IMAGE_NAME="${ANALYTICS_IMAGE_NAME:-klyster-analytics}"
ANALYTICS_IMAGE_TAG="${ANALYTICS_IMAGE_TAG:-latest}"

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

build_analytics() {
    echo "Building Klyster analytics image: ${ANALYTICS_IMAGE_NAME}:${ANALYTICS_IMAGE_TAG}"
    docker build \
        -f build/Dockerfile.analytics \
        -t "${ANALYTICS_IMAGE_NAME}:${ANALYTICS_IMAGE_TAG}" \
        .
    echo "✅ Analytics build complete: ${ANALYTICS_IMAGE_NAME}:${ANALYTICS_IMAGE_TAG}"
}

case "$TARGET" in
    core)
        build_core
        ;;
    analytics)
        build_analytics
        ;;
    all)
        build_core
        build_analytics
        ;;
    *)
        echo "Usage: $0 [core|analytics|all]" >&2
        exit 1
        ;;
esac

echo ""
echo "Run with docker-compose:"
echo "  cd build && docker-compose up"
