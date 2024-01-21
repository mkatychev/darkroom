# Run clippy with the --fix flag
fix:
  cd {{invocation_directory()}}; cargo clippy --fix --all-targets --all-features

# Run nightly cargo format
fmt:
  cargo +nightly fmt --all
