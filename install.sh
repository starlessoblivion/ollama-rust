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

    # SMART RUST CHECK: Avoids conflict with rustup
    if command -v cargo &> /dev/null; then
        echo "Rust detected. Skipping rust package to prevent conflicts..."
        PKGS="git binutils base-devel openssl"
    else
        PKGS="git rust binutils base-devel openssl"
    fi
    OPENSSL_PATH="/usr"
else
    echo "Unsupported environment. This script supports Termux and Arch Linux."
    exit 1
fi

echo "Detected System: $OS"
echo "Updating and installing dependencies..."
eval "$UPDATE_CMD"
eval "$INSTALL_CMD $PKGS"

# 1. Get the code
if [ ! -d "$HOME/ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git "$HOME/ollama-rust"
fi

# 2. Move into project directory
cd "$HOME/ollama-rust" || { echo "Failed to enter directory"; exit 1; }

# 3. Setup Environment Variables for Build
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS="-L$OPENSSL_PATH/lib"
export CPPFLAGS="-I$OPENSSL_PATH/include"

echo "Building project (this will take a minute)..."
cargo build --release

# 4. Create the shortcut
# We use \$ to ensure variables are evaluated when the shortcut RUNS
echo "#!/bin/bash
export OPENSSL_DIR=$OPENSSL_PATH
export LDFLAGS=\"-L$OPENSSL_PATH/lib\"
export CPPFLAGS=\"-I$OPENSSL_PATH/include\"
cd \$HOME/ollama-rust
./target/release/ollama-rust" > ~/run-ollama.sh

chmod +x ~/run-ollama.sh

# 5. Status Check
echo "-------------------------------------------"
echo "Checking Ollama status..."
if curl -s http://localhost:11434/api/tags > /dev/null; then
    echo "[OK] Ollama is running."
else
    echo "[!] Warning: Ollama is not detected. Run 'ollama serve' in another terminal."
fi

echo "-------------------------------------------"
echo "Setup Complete! Launch with: ./run-ollama.sh"
echo "-------------------------------------------"
