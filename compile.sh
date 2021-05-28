#!/usr/bin/env bash

set -ueo pipefail

docker info >/dev/null

VERSION="$(git describe --tags --abbrev=8)"

# TARGETS=(aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl)
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)

build_cross() {
  for target in $TARGETS; do
    cross build --target $target --release
    tar czf "darkroom-${VERSION}-${target}.tar.gz" -C "./target/${target}/release" dark
    cross build --target $target --release --no-default-features
    tar czf "darkroom-${VERSION}-${target}-no-default-features.tar.gz" -C "./target/${target}/release" dark
  done
}

build_linux() {
  local target="x86_64-unknown-linux-musl"
  docker run --platform linux/amd64 --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release
  tar czf "darkroom-${VERSION}-${target}.tar.gz" -C "./target/${target}/release" dark
  docker run --platform linux/amd64 --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release --no-default-features
  tar czf "darkroom-${VERSION}-${target}-no-default-features.tar.gz" -C "./target/${target}/release" dark
}

build_linux
