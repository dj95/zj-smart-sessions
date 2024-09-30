[private]
default:
    just --choose

# Build the wasm file
build:
  cargo build --features tracing

# Build and run the plugin
run: build
    zellij plugin \
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
