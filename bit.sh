#!/bin/sh
#
# Use this script to run your program LOCALLY.
#
set -e # Exit early if any commands fail

(
  cd "$(dirname "$0")" # Ensure compile steps are run within the repository directory
  rm -rf /tmp/bit || true
  cargo build --release --target-dir=/tmp/bit --manifest-path Cargo.toml
)

exec /tmp/bit/release/bit "$@"
