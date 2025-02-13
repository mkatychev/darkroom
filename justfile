# Justfiles are processed by the just command runner (https://just.systems/).

# You can install it with `brew install just` or `cargo install just`
_default:
    just --list

# Run rustfmt
fmt:
    rustup run nightly cargo fmt -- \
      --config-path ./fmt/rustfmt.toml

# Run clippy fix and rustfmt afterwards
fix *args: && fmt
    cd {{ invocation_directory() }}; cargo clippy --fix --all-targets --all-features {{ args }}

integration:
  cargo run -- -v record ./test_data grpc
  cargo run -- record ./test_data post
  cargo run -- take ./test_data/post.01s.body.fr.json
  cargo run -- vrecord ./test_data/alt_post.vr.json
  cargo run -- vrecord ./test_data/post.vr.json

record reel:
  cargo run -- record ./test_data {{reel}}
