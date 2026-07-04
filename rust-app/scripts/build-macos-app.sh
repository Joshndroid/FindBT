#!/usr/bin/env bash
set -euo pipefail

CONFIGURATION="${1:-release}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_NAME="FindBT"
BINARY_NAME="findbt-app"
APP_BINARY_NAME="FindBT"
if [[ "${CONFIGURATION}" == "debug" ]]; then
  CARGO_ARGS=(build -p findbt-app)
  TARGET_DIR="${ROOT}/target/debug"
  DIST_KIND="debug"
elif [[ "${CONFIGURATION}" == "release" ]]; then
  CARGO_ARGS=(build -p findbt-app --release)
  TARGET_DIR="${ROOT}/target/release"
  DIST_KIND="release"
else
  echo "Usage: scripts/build-macos-app.sh [debug|release]" >&2
  exit 2
fi
APP_DIR="${ROOT}/dist/macos/${DIST_KIND}/${APP_NAME}.app"
CONTENTS_DIR="${APP_DIR}/Contents"
MACOS_DIR="${CONTENTS_DIR}/MacOS"
RESOURCES_DIR="${CONTENTS_DIR}/Resources"

cd "${ROOT}"
cargo "${CARGO_ARGS[@]}"

rm -rf "${APP_DIR}"
mkdir -p "${MACOS_DIR}" "${RESOURCES_DIR}"

cp "${TARGET_DIR}/${BINARY_NAME}" "${MACOS_DIR}/${APP_BINARY_NAME}"
cp "${ROOT}/macos/Info.plist" "${CONTENTS_DIR}/Info.plist"
cp "${ROOT}/assets/icons/FindBT.icns" "${RESOURCES_DIR}/FindBT.icns"
chmod 755 "${MACOS_DIR}/${APP_BINARY_NAME}"

echo "Built ${APP_DIR}"
echo "For release distribution, sign and notarize this bundle after hardware verification."
