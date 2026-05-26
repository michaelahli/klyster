#!/bin/bash
# Build Klyster Docker image

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE_NAME="${IMAGE_NAME:-klyster}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

echo "Building Klyster Docker image..."
echo "Image: ${IMAGE_NAME}:${IMAGE_TAG}"

cd "$PROJECT_ROOT"

docker build \
    -f build/Dockerfile \
    -t "${IMAGE_NAME}:${IMAGE_TAG}" \
    .

echo "✅ Build complete: ${IMAGE_NAME}:${IMAGE_TAG}"
echo ""
echo "Run standalone:"
echo "  docker run -p 8080:8080 ${IMAGE_NAME}:${IMAGE_TAG}"
echo ""
echo "Run with docker-compose:"
echo "  cd build && docker-compose up"
