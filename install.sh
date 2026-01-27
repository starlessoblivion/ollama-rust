#!/bin/bash
# Ollama-Rust Universal Installer (Leptos Fullstack Edition)

# 1. Environment Detection
if command -v pkg &> /dev/null && [ -d "/data/data/com.termux" ]; then
    OS="termux"
    INSTALL_CMD="pkg install -y"
    UPDATE_CMD="pkg update -y && pkg upgrade -y"
    PKGS="git rust binutils build-essential openssl openssl-tool binaryen"
    OPENSSL_PATH=$PREFIX
elif command -v pacman &> /dev/null; then
    OS="arch"
    INSTALL_CMD="sudo pacman -S --needed --noconfirm"
    UPDATE_CMD="sudo pacman -Syu --noconfirm"
    PKGS="git rust binutils base-devel openssl binaryen"
    OPENSSL_PATH="/usr"
else
    echo "Unsupported environment."
    exit 1
fi

echo "Detected System: $OS"
eval "$UPDATE_CMD" || true
eval "$INSTALL_CMD $PKGS"

# 2. WASM Setup
rustup target add wasm32-unknown-unknown 2>/dev/null

if ! command -v cargo-leptos &> /dev/null; then
    cargo install --locked cargo-leptos
fi

# 3. Repository Setup
if [ ! -d "$HOME/ollama-rust" ]; then
    git clone https://github.com/starlessoblivion/ollama-rust.git "$HOME/ollama-rust"
    cd "$HOME/ollama-rust"
else
    cd "$HOME/ollama-rust" && git pull origin main
fi

# 4. Patching (Clean logic)
echo "Ensuring stable configuration..."
# Remove nightly feature if it exists in Cargo.toml
sed -i 's/features = \["nightly"\]//g' Cargo.toml

# 5. Compilation
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS="-L$OPENSSL_PATH/lib"
export CPPFLAGS="-I$OPENSSL_PATH/include"

echo "Building project..."
cargo leptos build --release

# 6. Launch Shortcut
echo "#!/bin/bash
export LEPTOS_SITE_ROOT=\"\$HOME/ollama-rust/target/site\"
cd \$HOME/ollama-rust
./target/release/ollama-rust" > ~/run-ollama.sh
chmod +x ~/run-ollama.sh

echo "Setup Complete! Launch with: ~/run-ollama.sh"
