#!/bin/bash
# Ollama-Rust Universal Installer (Leptos Fullstack Edition)
# Supports: Termux, Arch, Fedora, Ubuntu, Debian, Raspberry Pi OS

# 1. Environment Detection & Package Configuration
if command -v pkg &> /dev/null && [ -d "/data/data/com.termux" ]; then
    OS="termux"
    INSTALL_CMD="pkg install -y"
    UPDATE_CMD="pkg update -y && pkg upgrade -y"
    # Added binaryen for WASM optimization
    PKGS="git rust binutils build-essential openssl openssl-tool binaryen"
    OPENSSL_PATH=$PREFIX
elif command -v pacman &> /dev/null; then
    OS="arch"
    INSTALL_CMD="sudo pacman -S --needed --noconfirm"
    UPDATE_CMD="sudo pacman -Syu --noconfirm"
    # RESTORED: Only install 'rust' if 'cargo' is missing to avoid rustup conflicts
    if command -v cargo &> /dev/null; then
        PKGS="git binutils base-devel openssl binaryen"
    else
        PKGS="git rust binutils base-devel openssl binaryen"
    fi
    OPENSSL_PATH="/usr"
elif command -v dnf &> /dev/null; then
    OS="fedora"
    INSTALL_CMD="sudo dnf install -y"
    UPDATE_CMD="sudo dnf check-update"
    PKGS="git rust cargo binutils openssl-devel @development-tools binaryen"
    OPENSSL_PATH="/usr"
elif command -v apt-get &> /dev/null; then
    OS="debian/ubuntu/raspberrypi"
    INSTALL_CMD="sudo apt-get install -y"
    UPDATE_CMD="sudo apt-get update"
    PKGS="git rustc cargo binutils build-essential libssl-dev pkg-config binaryen"
    OPENSSL_PATH="/usr"
else
    echo "Unsupported environment."
    exit 1
fi

echo "-------------------------------------------"
echo "Detected System: $OS"
echo "Updating and installing dependencies..."
echo "-------------------------------------------"

# Run update and install
eval "$UPDATE_CMD" || true
eval "$INSTALL_CMD $PKGS"

# 2. WASM Toolchain Setup
echo "Configuring Rust for Fullstack (WASM)..."
# Ensure the wasm target is present for the frontend
rustup target add wasm32-unknown-unknown 2>/dev/null || echo "WASM target already present or rustup not used."

if ! command -v cargo-leptos &> /dev/null; then
    echo "Installing cargo-leptos..."
    cargo install --locked cargo-leptos
fi

# 3. Repository Setup
if [ ! -d "$HOME/ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git "$HOME/ollama-rust"
else
    echo "Existing repository found. Pulling latest changes..."
    cd "$HOME/ollama-rust" && git pull origin main
fi

cd "$HOME/ollama-rust" || { echo "Failed to enter directory"; exit 1; }

# Ensure folder structure is correct for Leptos
mkdir -p public
[ -f style.css ] && mv style.css public/

# 4. Compilation
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS="-L$OPENSSL_PATH/lib"
export CPPFLAGS="-I$OPENSSL_PATH/include"

echo "Building Fullstack project (Release mode)..."
# This compiles the server binary AND the WASM frontend
cargo leptos build --release

# 5. Create Launch Shortcut
# Added LEPTOS_SITE_ROOT so the binary knows where the WASM/CSS files are
echo "#!/bin/bash
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS=\"-L$OPENSSL_PATH/lib\"
export CPPFLAGS=\"-I$OPENSSL_PATH/include\"
export LEPTOS_SITE_ROOT=\"\$HOME/ollama-rust/target/site\"
cd \$HOME/ollama-rust
./target/release/ollama-rust" > ~/run-ollama.sh

chmod +x ~/run-ollama.sh

# 6. Final Status Check
echo "-------------------------------------------"
if curl -s http://localhost:11434/api/tags > /dev/null; then
    echo "[OK] Ollama is running."
else
    echo "[!] Warning: Ollama backend not detected."
fi

echo "-------------------------------------------"
echo "Setup Complete!"
echo "Launch with: ~/run-ollama.sh"
echo "-------------------------------------------"
