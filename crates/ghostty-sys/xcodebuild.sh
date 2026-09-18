#!/bin/sh
set -eu

create=0
for arg in "$@"; do
  if [ "$arg" = "-create-xcframework" ]; then
    create=1
    break
  fi
done
if [ "$create" -eq 0 ]; then
  exec /usr/bin/xcodebuild "$@"
fi

library=""
output=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -library)
      library=$2
      shift 2
      ;;
    -output)
      output=$2
      shift 2
      ;;
    -headers|-debug-symbols)
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done

if [ -z "$library" ] || [ -z "$output" ]; then
  exit 1
fi

slice="$output/macos-arm64"
mkdir -p "$slice"
cp "$library" "$slice/$(basename "$library")"
