#!/usr/bin/env bash

VERSION="0.7.0-a"

cargo build --release &&
  tar czf darkroom-"$VERSION"-x86_64-apple-darwin.tar.gz -C target/release dark &&
  cargo build --release --no-default-features &&
  tar czf darkroom-"$VERSION"-x86_64-apple-darwin-no-features.tar.gz -C target/release dark &&
  docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release &&
  tar czf darkroom-"$VERSION"-x86_64-unknown-linux-musl.tar.gz -C ./target/x86_64-unknown-linux-musl/release dark &&
  docker run --rm -it -v "$(pwd)":/home/rust/src ekidd/rust-musl-builder cargo build --release --no-default-features &&
  tar czf darkroom-"$VERSION"-x86_64-unknown-linux-musl-no-features.tar.gz -C ./target/x86_64-unknown-linux-musl/release dark
