#!/bin/sh
# Build src-lib in debug mode
cargo build -p src-lib
if [ $? -ne 0 ]; then exit 1; fi

# Build src-sidecar for current platform in debug mode
cargo build --bin src-sidecar
if [ $? -ne 0 ]; then exit 1; fi

# Detect platform and copy to what Tauri expects for dev
ARCH=$(uname -m)
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$OS" = "darwin" ]; then
  TARGET_TRIPLE="${ARCH}-apple-darwin"
elif [ "$OS" = "linux" ]; then
  TARGET_TRIPLE="${ARCH}-unknown-linux-gnu"
else
  echo "Unsupported OS: $OS"
  exit 1
fi

mkdir -p target/debug
cp "src-sidecar/target/debug/src-sidecar" "target/release/src-sidecar-${TARGET_TRIPLE}"

# Run Tauri dev
bun tauri dev