#!/bin/bash

# 1. Update Termux packages
echo "Updating packages..."
pkg update -y && pkg upgrade -y

# 2. Install dependencies
echo "Installing Rust, Git, and Build Essentials..."
pkg install -y rust binutils git build-essential openssl openssl-tool

# 3. Setup OpenSSL paths (Crucial for Rust on Termux)
export OPENSSL_DIR=$PREFIX
export LDFLAGS="-L$PREFIX/lib"
export CPPFLAGS="-I$PREFIX/include"

# 4. Build the project
echo "Building ollama-rust..."
cargo build --release

echo "-------------------------------------------"
echo "Setup Complete! Run the server with:"
echo "./target/release/ollama-rust"
echo "-------------------------------------------"
