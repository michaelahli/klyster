#!/bin/bash
# Automated clippy fix script
# Run with: bash fix_clippy.sh

set -e

echo "Running clippy with auto-fixes..."

# Fix specific warnings with cargo clippy --fix
cargo clippy --fix --allow-dirty --allow-staged --lib --all-targets -- \
  -A clippy::cast_lossless \
  -A clippy::cast_possible_truncation \
  -A clippy::cast_possible_wrap \
  -A clippy::cast_precision_loss \
  -A clippy::cast_sign_loss \
  -A clippy::doc_markdown \
  -A clippy::missing_panics_doc \
  -A clippy::return_self_not_must_use \
  -A missing_docs

echo "Auto-fixes applied. Running clippy again to check remaining issues..."
cargo clippy --all-targets --all-features --lib -- -D warnings

echo "Done!"
