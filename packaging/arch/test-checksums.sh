#!/usr/bin/env bash
set -euo pipefail

recipe=$(cd -- "$(dirname -- "$0")" && pwd)/PKGBUILD
valid_lower=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
valid_upper=ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789

source_digest=$(env SOLITAIRE_SOURCE_ARCHIVE=fixture.tar.gz \
  SOLITAIRE_SOURCE_SHA256="$valid_upper" \
  bash -c 'source "$1"; printf "%s" "${sha256sums[0]}"' _ "$recipe")
[[ $source_digest == "${valid_upper,,}" ]]

default_release_digest=$(bash -c \
  'source "$1"; printf "%s" "${sha256sums[0]}"' _ "$recipe")
[[ $default_release_digest == \
  6db5400d5d384302d43bb218618468233ab27f850e76580f21fb46d25fac43bf ]]

release_digest=$(env SOLITAIRE_RELEASE_SHA256="$valid_lower" \
  bash -c 'source "$1"; printf "%s" "${sha256sums[0]}"' _ "$recipe")
[[ $release_digest == "$valid_lower" ]]

expect_rejected() {
  local variable=$1
  local value=$2
  shift 2
  if env "$variable=$value" "$@" bash -c 'source "$1"' _ "$recipe" 2>/dev/null; then
    printf 'accepted invalid %s value: %q\n' "$variable" "$value" >&2
    exit 1
  fi
}

for invalid in '' SKIP abc 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde \
  0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0 \
  z123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef; do
  expect_rejected SOLITAIRE_SOURCE_SHA256 "$invalid" \
    env SOLITAIRE_SOURCE_ARCHIVE=fixture.tar.gz
  expect_rejected SOLITAIRE_RELEASE_SHA256 "$invalid" env
done
