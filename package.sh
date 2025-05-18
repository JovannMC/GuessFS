#!/bin/sh
# Build src-lib first
cargo build --release -p src-lib
if [ $? -ne 0 ]; then exit 1; fi

# Detect platform and build src-sidecar for the correct target
case "$(uname -s)" in
    Darwin)
        cargo build --release --bin src-sidecar-x86_64-apple-darwin --target x86_64-apple-darwin
        cp target/x86_64-apple-darwin/release/src-sidecar-x86_64-apple-darwin target/release/src-sidecar-x86_64-apple-darwin
        ;;
    Linux)
        cargo build --release --bin src-sidecar --target x86_64-unknown-linux-gnu
        cp target/x86_64-unknown-linux-gnu/release/src-sidecar target/release/src-sidecar-x86_64-unknown-linux-gnu
        ;;
    *)
        echo "Unsupported OS"
        exit 1
        ;;
esac

if [ $? -ne 0 ]; then exit 1; fi

# Build Tauri app
bun tauri build