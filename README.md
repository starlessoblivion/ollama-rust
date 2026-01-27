# Ollama-Rust Mobile

A lightweight Rust-based web interface for Ollama, optimized for desktop and mobile browsers.

## Android (Termux) Quick Start

To set up the environment, compile, and run on Android, copy and paste this command into Termux:

```bash
pkg update -y && pkg upgrade -y && pkg install -y git rust binutils build-essential openssl openssl-tool && export OPENSSL_DIR=$PREFIX && export LDFLAGS="-L$PREFIX/lib" && export CPPFLAGS="-I$PREFIX/include" && git clone https://github.com/starlessoblivion/ollama-rust.git && cd ollama-rust && cargo build --release
