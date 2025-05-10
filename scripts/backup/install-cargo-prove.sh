#!/usr/bin/env bash
# This script downloads the cargo-prove binary for the current platform

set -e

PRJ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLATFORM="$(uname -s)"
ARCH="$(uname -m)"

# Create the bin directory if it doesn't exist
mkdir -p "${PRJ_ROOT}/bin"

# Define the version to use
SP1_VERSION="v4.2.0"

# Determine the correct archive name based on platform and architecture
if [ "${PLATFORM}" = "Darwin" ]; then
  if [ "${ARCH}" = "arm64" ]; then
    PLATFORM_TARGET="darwin_arm64"
  else
    PLATFORM_TARGET="darwin_amd64"
  fi
elif [ "${PLATFORM}" = "Linux" ]; then
  if [ "${ARCH}" = "aarch64" ]; then
    PLATFORM_TARGET="linux_arm64"
  else
    PLATFORM_TARGET="linux_amd64"
  fi
else
  echo "Unsupported platform: ${PLATFORM}"
  exit 1
fi

ARCHIVE_NAME="cargo_prove_${SP1_VERSION}_${PLATFORM_TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/succinctlabs/sp1/releases/download/${SP1_VERSION}/${ARCHIVE_NAME}"

echo "Installing cargo-prove for ${PLATFORM_TARGET}"
echo "Downloading from: ${DOWNLOAD_URL}"

# Create a temporary directory for extraction
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Download the archive
curl -L "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ARCHIVE_NAME}" --progress-bar

# Extract the archive
tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "${TMP_DIR}"

# Copy the binary to our bin directory
cp "${TMP_DIR}/cargo-prove" "${PRJ_ROOT}/bin/"

# Make it executable
chmod +x "${PRJ_ROOT}/bin/cargo-prove"

# Verify that it works
echo "Testing cargo-prove:"
"${PRJ_ROOT}/bin/cargo-prove" prove --version || "${PRJ_ROOT}/bin/cargo-prove"

echo "cargo-prove has been successfully installed to ${PRJ_ROOT}/bin/cargo-prove" 