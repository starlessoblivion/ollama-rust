#!/bin/bash
# Ollama-Rust Universal Installer
# Supports: Termux (Android), Arch Linux, Debian/Ubuntu, Fedora

set -e

echo "=== Ollama-Rust Installer ==="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

# 1. Environment Detection
detect_os() {
    if command -v pkg &> /dev/null && [ -d "/data/data/com.termux" ]; then
        echo "termux"
    elif [ -f /etc/arch-release ] || command -v pacman &> /dev/null; then
        echo "arch"
    elif [ -f /etc/debian_version ] || command -v apt-get &> /dev/null; then
        echo "debian"
    elif [ -f /etc/fedora-release ] || command -v dnf &> /dev/null; then
        echo "fedora"
    elif [ -f /etc/redhat-release ] || command -v yum &> /dev/null; then
        echo "rhel"
    else
        echo "unknown"
    fi
}

OS=$(detect_os)
info "Detected System: $OS"

# 2. Install system dependencies
install_deps() {
    case $OS in
        termux)
            info "Updating Termux packages..."
            pkg update -y && pkg upgrade -y
            info "Installing dependencies..."
            pkg install -y git binutils build-essential openssl openssl-tool binaryen
            export OPENSSL_DIR=$PREFIX
            export LDFLAGS="-L$PREFIX/lib"
            export CPPFLAGS="-I$PREFIX/include"
            ;;
        arch)
            info "Updating Arch packages..."
            sudo pacman -Syu --noconfirm || true
            info "Installing dependencies..."
            # Don't install rust package - use rustup instead to avoid conflicts
            sudo pacman -S --needed --noconfirm git binutils base-devel openssl binaryen || true
            export OPENSSL_DIR="/usr"
            ;;
        debian)
            info "Updating Debian/Ubuntu packages..."
            sudo apt-get update
            info "Installing dependencies..."
            sudo apt-get install -y git build-essential pkg-config libssl-dev binaryen
            export OPENSSL_DIR="/usr"
            ;;
        fedora)
            info "Updating Fedora packages..."
            sudo dnf check-update || true
            info "Installing dependencies..."
            sudo dnf install -y git gcc make openssl-devel binaryen
            export OPENSSL_DIR="/usr"
            ;;
        rhel)
            info "Installing RHEL/CentOS dependencies..."
            sudo yum install -y git gcc make openssl-devel
            export OPENSSL_DIR="/usr"
            ;;
        *)
            warn "Unknown OS. Assuming dependencies are installed."
            export OPENSSL_DIR="/usr"
            ;;
    esac
}

# 3. Install Rust via rustup if not present
install_rust() {
    if command -v rustup &> /dev/null; then
        info "Rustup already installed, updating..."
        rustup update stable || true
    elif command -v cargo &> /dev/null; then
        # Cargo exists but not rustup - might be system rust
        warn "Cargo found but not rustup. Using existing installation."
    else
        info "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        source "$HOME/.cargo/env"
    fi

    # Ensure cargo is in PATH
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi
}

# 4. Setup WASM target and cargo-leptos
setup_wasm() {
    info "Adding WASM target..."
    rustup target add wasm32-unknown-unknown 2>/dev/null || true

    if ! command -v cargo-leptos &> /dev/null; then
        info "Installing cargo-leptos (this may take a while)..."
        cargo install --locked cargo-leptos
    else
        info "cargo-leptos already installed"
    fi
}

# 5. Clone or update repository
setup_repo() {
    REPO_DIR="$HOME/ollama-rust"

    if [ ! -d "$REPO_DIR" ]; then
        info "Cloning repository..."
        git clone https://github.com/starlessoblivion/ollama-rust.git "$REPO_DIR"
    else
        info "Updating existing repository..."
        cd "$REPO_DIR"
        git fetch origin main
        git reset --hard origin/main
    fi

    cd "$REPO_DIR"
}

# 6. Build the project
build_project() {
    info "Building project (this may take a while on first run)..."

    # Export OpenSSL paths
    export OPENSSL_DIR=${OPENSSL_DIR:-/usr}
    export PKG_CONFIG_PATH="$OPENSSL_DIR/lib/pkgconfig:$PKG_CONFIG_PATH"

    cargo leptos build --release
}

# 7. Create launcher script
create_launcher() {
    LAUNCHER="$HOME/run-ollama.sh"

    cat > "$LAUNCHER" << 'LAUNCHER_EOF'
#!/bin/bash
export LEPTOS_SITE_ROOT="$HOME/ollama-rust/target/site"
cd "$HOME/ollama-rust"
exec ./target/release/ollama-rust
LAUNCHER_EOF

    chmod +x "$LAUNCHER"
    info "Launcher created: $LAUNCHER"
}

# Main installation flow
main() {
    install_deps
    install_rust
    setup_wasm
    setup_repo
    build_project
    create_launcher

    echo ""
    echo -e "${GREEN}=== Setup Complete! ===${NC}"
    echo ""
    echo "To start the server, run:"
    echo "  ~/run-ollama.sh"
    echo ""
    echo "Then open http://localhost:3000 in your browser"
    echo "(or http://127.0.0.1:3000 on Android)"
}

main "$@"
