[private]
default:
    just --choose

# Build the wasm file
build:
  cargo build --features tracing

# Build and run the plugin
run: build
    zellij plugin \
        --configuration "find_command=$PWD/find_command" \
        --skip-plugin-cache \
        --floating \
        -- file:./target/wasm32-wasip1/debug/zj-smart-sessions.wasm

# Watch and run tests with nextest.
test:
  cargo watch -x "nextest run --lib"

# Lint with clippy and cargo audit.
lint:
  cargo clippy --all-features --lib
  cargo audit

# Create and push a new release version.
release:
  #!/usr/bin/env bash
  export VERSION="$( git cliff --bumped-version )"
  cargo set-version "${VERSION:1}"
  direnv exec . cargo build --release
  git commit -am "chore: bump version to $VERSION"
  git tag -m "$VERSION" "$VERSION"
  git push origin main
  git push origin "$VERSION"
