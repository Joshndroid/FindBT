#!/usr/bin/env bash
set -euo pipefail
export COPYFILE_DISABLE=1

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERSION="$(awk -F '"' 'found && /^version = / { print $2; exit } /^\[workspace.package\]/ { found=1 }' "${ROOT}/Cargo.toml")"
APP_NAME="FindBT"
RELEASE="FindBT-v${VERSION}-macos"
DIST_DIR="${ROOT}/dist/macos"
RELEASE_DIR="${DIST_DIR}/release"
ARTIFACT_DIR="${DIST_DIR}/artifacts"
APP_DIR="${RELEASE_DIR}/${APP_NAME}.app"
PORTABLE_STAGING="${DIST_DIR}/${RELEASE}-portable"
OFFLINE_STAGING="${DIST_DIR}/${RELEASE}-offline-portable"
LOCAL_RELEASE="${ARTIFACT_DIR}/local-release.txt"

cd "${ROOT}"
"${ROOT}/scripts/build-macos-app.sh" release

rm -rf "${ARTIFACT_DIR}" "${PORTABLE_STAGING}" "${OFFLINE_STAGING}"
mkdir -p "${ARTIFACT_DIR}" "${PORTABLE_STAGING}" "${OFFLINE_STAGING}"

xattr -cr "${APP_DIR}" 2>/dev/null || true

if [[ -n "${MACOS_CODESIGN_IDENTITY:-}" ]]; then
  echo "Signing ${APP_DIR}"
  codesign --force --deep --options runtime --timestamp --sign "${MACOS_CODESIGN_IDENTITY}" "${APP_DIR}"
  codesign --verify --deep --strict --verbose=2 "${APP_DIR}"
fi

PKG_PATH="${ARTIFACT_DIR}/${RELEASE}-installer.pkg"
PRODUCTBUILD_ARGS=(--component "${APP_DIR}" /Applications)
if [[ -n "${MACOS_INSTALLER_SIGNING_IDENTITY:-}" ]]; then
  PRODUCTBUILD_ARGS=(--sign "${MACOS_INSTALLER_SIGNING_IDENTITY}" "${PRODUCTBUILD_ARGS[@]}")
fi
productbuild "${PRODUCTBUILD_ARGS[@]}" "${PKG_PATH}"

if [[ "${MACOS_NOTARIZE:-0}" == "1" ]]; then
  if [[ -z "${APPLE_ID:-}" || -z "${APPLE_TEAM_ID:-}" || -z "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]]; then
    echo "MACOS_NOTARIZE=1 requires APPLE_ID, APPLE_TEAM_ID, and APPLE_APP_SPECIFIC_PASSWORD." >&2
    exit 2
  fi

  echo "Submitting ${PKG_PATH} for notarization"
  xcrun notarytool submit "${PKG_PATH}" \
    --apple-id "${APPLE_ID}" \
    --team-id "${APPLE_TEAM_ID}" \
    --password "${APPLE_APP_SPECIFIC_PASSWORD}" \
    --wait
  xcrun stapler staple "${PKG_PATH}"
fi

ditto --norsrc "${APP_DIR}" "${PORTABLE_STAGING}/${APP_NAME}.app"
cp "${ROOT}/../QUICKSTART.md" "${PORTABLE_STAGING}/quickstart.txt"
BUILT_UTC="$(date -u '+%Y-%m-%d %H:%M:%S UTC')"
cat > "${LOCAL_RELEASE}" <<EOF
FindBT Local Release
====================

Version: v${VERSION}
Platform: macOS
Built: ${BUILT_UTC}

Artifacts:
- ${RELEASE}-installer.pkg
- ${RELEASE}-portable.zip
- ${RELEASE}-offline-portable.zip

SHA256 files are generated beside each installer and portable zip.

Notes:
- The app is built as a native macOS .app bundle.
- The bundle contains Bluetooth privacy usage strings.
- Signing identity: ${MACOS_CODESIGN_IDENTITY:-not used}
- Installer signing identity: ${MACOS_INSTALLER_SIGNING_IDENTITY:-not used}
- Notarization requested: ${MACOS_NOTARIZE:-0}
EOF
cp "${LOCAL_RELEASE}" "${PORTABLE_STAGING}/local-release.txt"
cat >> "${PORTABLE_STAGING}/local-release.txt" <<EOF

FindBT macOS Portable
=====================

Contents:
- ${APP_NAME}.app
- quickstart.txt
- local-release.txt

This portable app bundle is self-contained for offline runtime use. Release distribution should be signed and notarized after hardware verification.
EOF
ditto --norsrc -c -k --keepParent "${PORTABLE_STAGING}" "${ARTIFACT_DIR}/${RELEASE}-portable.zip"

ditto --norsrc "${APP_DIR}" "${OFFLINE_STAGING}/${APP_NAME}.app"
cp "${ROOT}/../QUICKSTART.md" "${OFFLINE_STAGING}/quickstart.txt"
cp "${LOCAL_RELEASE}" "${OFFLINE_STAGING}/local-release.txt"
cat > "${OFFLINE_STAGING}/offline-readme.txt" <<EOF
FindBT macOS Offline Portable
=============================

This package is intended for offline machines. It contains the built ${APP_NAME}.app and operator notes.
No network access is required to run the app.
EOF
ditto --norsrc -c -k --keepParent "${OFFLINE_STAGING}" "${ARTIFACT_DIR}/${RELEASE}-offline-portable.zip"

for artifact in "${ARTIFACT_DIR}"/*; do
  if [[ -f "${artifact}" && "${artifact}" != *.sha256.txt && "$(basename "${artifact}")" != "local-release.txt" ]]; then
    shasum -a 256 "${artifact}" > "${artifact}.sha256.txt"
  fi
done

echo "macOS artifacts:"
ls -1 "${ARTIFACT_DIR}"
