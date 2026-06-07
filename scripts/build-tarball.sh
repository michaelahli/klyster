#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -lt 3 ]]; then
    echo "Usage: $0 <binary-path> <version> <platform> [output-dir]" >&2
    exit 1
fi

binary_path="$1"
version="$2"
platform="$3"
output_dir="${4:-dist}"

if [[ ! -f "${binary_path}" ]]; then
    echo "Binary not found: ${binary_path}" >&2
    exit 1
fi

package_name="klyster-${version}-${platform}"
staging_dir="${output_dir}/${package_name}"
archive_path="${output_dir}/${package_name}.tar.gz"

rm -rf "${staging_dir}"
mkdir -p "${staging_dir}/bin" "${staging_dir}/config"

cp "${binary_path}" "${staging_dir}/bin/klyster"
chmod +x "${staging_dir}/bin/klyster"

cp README.md LICENSE "${staging_dir}/"
cp klyster.example.toml agent.example.toml "${staging_dir}/config/"

cat > "${staging_dir}/README.install.md" <<EOF
# Klyster ${version} ${platform}

This archive contains the Klyster binary and example configuration files.

## Run

\`\`\`bash
./bin/klyster --config ./config/klyster.example.toml
\`\`\`

Copy and edit the example configuration before running Klyster in production.
EOF

tar -czf "${archive_path}" -C "${output_dir}" "${package_name}"
rm -rf "${staging_dir}"

printf '%s\n' "${archive_path}"
