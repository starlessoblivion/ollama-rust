#!/bin/bash
# Ollama-Rust Universal Installer

# Detect Environment
if command -v pkg &> /dev/null && [ -d "/data/data/com.termux" ]; then
    OS="termux"
    INSTALL_CMD="pkg install -y"
    UPDATE_CMD="pkg update -y && pkg upgrade -y"
    PKGS="git rust binutils build-essential openssl openssl-tool"
    OPENSSL_PATH=$PREFIX
elif command -v pacman &> /dev/null; then
    OS="arch"
    INSTALL_CMD="sudo pacman -S --needed --noconfirm"
    UPDATE_CMD="sudo pacman -Syu --noconfirm"

    # SMART RUST CHECK:
    # If 'cargo' or 'rustc' already exists, don't try to install the 'rust' package
    if command -v cargo &> /dev/null; then
        echo "Rust is already installed (via rustup or pacman). Skipping rust package..."
        PKGS="git binutils base-devel openssl"
    else
        PKGS="git rust binutils base-devel openssl"
    fi
    OPENSSL_PATH="/usr"
else
    echo "Unsupported environment."
    exit 1
fi
