#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

APP_NAME="Combe"
BUNDLE_ID="com.samzong.combe"
BUILD_DIR="build"
APP_DIR="${BUILD_DIR}/${APP_NAME}.app"
BIN_NAME="combe"
VERSION="${1:-}"

if [[ $# -gt 0 && ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "error: version must be MAJOR.MINOR.PATCH" >&2
  exit 1
fi

cargo build --locked --release -p combe

BIN_PATH="target/release/${BIN_NAME}"
if [[ ! -f "${BIN_PATH}" ]]; then
  echo "error: built binary not found at ${BIN_PATH}" >&2
  exit 1
fi

RESOURCES_DIR=$(ls -td target/release/build/ghostty-sys-*/out/ghostty/share/ghostty 2>/dev/null | head -1)
if [[ -z "${RESOURCES_DIR}" || ! -d "${RESOURCES_DIR}" ]]; then
  echo "error: ghostty resources not found under target/release/build" >&2
  exit 1
fi

ICON_PATH="packaging/app-icon.icns"
if [[ ! -f "${ICON_PATH}" ]]; then
  echo "error: app icon not found at ${ICON_PATH}" >&2
  exit 1
fi

rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS" "${APP_DIR}/Contents/Resources"
cp "${BIN_PATH}" "${APP_DIR}/Contents/MacOS/${APP_NAME}"
cp packaging/Info.plist "${APP_DIR}/Contents/Info.plist"
if [[ -n "${VERSION}" ]]; then
  /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${VERSION}" "${APP_DIR}/Contents/Info.plist"
  /usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}" "${APP_DIR}/Contents/Info.plist"
fi
cp "${ICON_PATH}" "${APP_DIR}/Contents/Resources/app-icon.icns"
cp -R "${RESOURCES_DIR}" "${APP_DIR}/Contents/Resources/ghostty"

codesign --force --deep --sign - --identifier "${BUNDLE_ID}" "${APP_DIR}"

echo "${APP_DIR}"
