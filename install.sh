#!/bin/bash
# Ollama-Rust Universal Installer
# Supports: Termux, Arch, Fedora, Ubuntu, Debian, Raspberry Pi OS

# 1. Environment Detection & Package Configuration
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
    if command -v cargo &> /dev/null; then
        PKGS="git binutils base-devel openssl"
    else
        PKGS="git rust binutils base-devel openssl"
    fi
    OPENSSL_PATH="/usr"
elif command -v dnf &> /dev/null; then
    OS="fedora"
    INSTALL_CMD="sudo dnf install -y"
    UPDATE_CMD="sudo dnf check-update"
    if command -v cargo &> /dev/null; then
        PKGS="git binutils openssl-devel @development-tools"
    else
        PKGS="git rust cargo binutils openssl-devel @development-tools"
    fi
    OPENSSL_PATH="/usr"
elif command -v apt-get &> /dev/null; then
    OS="debian/ubuntu/raspberrypi"
    INSTALL_CMD="sudo apt-get install -y"
    UPDATE_CMD="sudo apt-get update"
    if command -v cargo &> /dev/null; then
        PKGS="git build-essential libssl-dev pkg-config"
    else
        PKGS="git rustc cargo build-essential libssl-dev pkg-config"
    fi
    OPENSSL_PATH="/usr"
else
    echo "Unsupported environment. This script supports Termux, Arch, Fedora, and Debian/Ubuntu."
    exit 1
fi

echo "-------------------------------------------"
echo "Detected System: $OS"
echo "Updating and installing dependencies..."
echo "-------------------------------------------"

# Run update and install
eval "$UPDATE_CMD" || true
eval "$INSTALL_CMD $PKGS"

# 2. Repository Setup
if [ ! -d "$HOME/ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git "$HOME/ollama-rust"
else
    echo "Existing repository found. Pulling latest changes..."
    cd "$HOME/ollama-rust" && git pull origin main
fi

cd "$HOME/ollama-rust" || { echo "Failed to enter directory"; exit 1; }

# 3. Environment Variables for Compilation
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS="-L$OPENSSL_PATH/lib"
export CPPFLAGS="-I$OPENSSL_PATH/include"

echo "Building project in release mode..."
cargo build --release

# 4. Create Launch Shortcut
echo "#!/bin/bash
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS=\"-L$OPENSSL_PATH/lib\"
export CPPFLAGS=\"-I$OPENSSL_PATH/include\"
cd \$HOME/ollama-rust
./target/release/ollama-rust" > ~/run-ollama.sh

chmod +x ~/run-ollama.sh

# 5. Final Status Check
echo "-------------------------------------------"
echo "Checking Ollama status..."
if curl -s http://localhost:11434/api/tags > /dev/null; then
    echo "[OK] Ollama is running."
else
    echo "[!] Warning: Ollama backend not detected."
    echo "    Make sure to run 'ollama serve' in another terminal."
fi

echo "-------------------------------------------"
echo "Setup Complete!"
echo "Launch the app with: ./run-ollama.sh"
echo "-------------------------------------------"
