#!/bin/bash
# Ollama-Rust Universal Installer (Leptos Fullstack Edition)

# 1. Environment Detection & Package Configuration
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

eval "$UPDATE_CMD" || true
eval "$INSTALL_CMD $PKGS"

# 2. WASM Toolchain & Dependencies Setup
echo "Configuring Rust for Fullstack (WASM)..."
rustup target add wasm32-unknown-unknown 2>/dev/null || echo "WASM target already present."

if ! command -v cargo-leptos &> /dev/null; then
    echo "Installing cargo-leptos..."
    cargo install --locked cargo-leptos
fi

# 3. Repository Setup
if [ ! -d "$HOME/ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git "$HOME/ollama-rust"
    cd "$HOME/ollama-rust"
else
    echo "Existing repository found. Pulling latest changes..."
    cd "$HOME/ollama-rust" && git pull origin main
fi

# --- NEW SECTION: PATCH SOURCE CODE AND DEPENDENCIES ---
echo "Patching project files and dependencies..."

# 3.1 Add missing crates to Cargo.toml if they aren't there
cargo add serde --features derive
cargo add wasm-bindgen
cargo add serde_json

# 3.2 Fix Struct Derives (E0277)
# Adds Serialize/Deserialize to StatusResponse and ChatMessage
sed -i 's/pub struct StatusResponse {/#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]\npub struct StatusResponse {/' src/app.rs
sed -i 's/pub struct ChatMessage {/#[derive(serde::Serialize, serde::Deserialize, Clone)]\npub struct ChatMessage {/' src/app.rs

# 3.3 Fix Leptos 0.7 Reactive Syntax (E0277 / E0599)
# Wraps 'input' in a closure for prop:value
sed -i 's/prop:value=input/prop:value=move || input.get()/' src/app.rs

# 3.4 Fix Action Dispatch Return Type (E0308)
# Adds a semicolon inside the closure to return ()
sed -i 's/toggle_action.dispatch(())/ { toggle_action.dispatch(()); }/' src/app.rs

# --- END PATCHING ---

# 4. Compilation
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS="-L$OPENSSL_PATH/lib"
export CPPFLAGS="-I$OPENSSL_PATH/include"

echo "Building Fullstack project (Release mode)..."
cargo leptos build --release

# 5. Create Launch Shortcut
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
