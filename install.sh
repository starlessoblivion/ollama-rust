#!/bin/bash
# Ollama Rust Web UI Installer v1.2.0
# Optimized for Termux/Android and standard Linux
# Fully non-interactive - no prompts
# Usage: curl -fsSL https://raw.githubusercontent.com/starlessoblivion/ollama-rust/main/install.sh | bash

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() { echo -e "${BLUE}[*]${NC} $1"; }
print_success() { echo -e "${GREEN}[✓]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[!]${NC} $1"; }
print_error() { echo -e "${RED}[✗]${NC} $1"; }

# Detect environment
IS_TERMUX=false
if [ -n "$TERMUX_VERSION" ] || [ -d "/data/data/com.termux" ]; then
    IS_TERMUX=true
fi

INSTALL_DIR="$HOME/ollama-rust"
REPO_URL="https://github.com/starlessoblivion/ollama-rust.git"

echo ""
echo -e "${GREEN}╔═══════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Ollama Rust Web UI Installer      ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════╝${NC}"
echo ""

if [ "$IS_TERMUX" = true ]; then
    print_status "Detected Termux environment"
else
    print_status "Detected standard Linux environment"
fi

# Install dependencies
install_dependencies() {
    print_status "Installing dependencies..."

    if [ "$IS_TERMUX" = true ]; then
        # Termux - install base deps
        pkg update -y
        pkg install -y git openssl pkg-config binutils
    elif command -v apt-get &> /dev/null; then
        # Debian/Ubuntu
        sudo apt-get update
        sudo apt-get install -y git curl build-essential pkg-config libssl-dev
    elif command -v pacman &> /dev/null; then
        # Arch Linux
        sudo pacman -Sy --noconfirm git curl base-devel openssl pkg-config
    elif command -v dnf &> /dev/null; then
        # Fedora
        sudo dnf install -y git curl gcc openssl-devel pkg-config
    else
        print_warning "Unknown package manager. Please install manually: git, openssl, pkg-config"
    fi

    print_success "Dependencies installed"
}

# Check and setup Rust
install_rust() {
    # Source cargo env first if it exists
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    # Check if rustup is available
    if command -v rustup &> /dev/null; then
        print_success "Using existing rustup: $(rustc --version)"
        return 0
    fi

    # Check if rustc exists without rustup
    if command -v rustc &> /dev/null; then
        print_warning "Found rustc $(rustc --version) but no rustup"
        print_warning "rustup is required to add the WASM target"
        print_status "Installing rustup alongside existing rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        fi
        print_success "Rustup installed"
        return 0
    fi

    # Neither rustup nor rustc found - install rustup
    print_status "No Rust installation found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

    # Source cargo env
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    print_success "Rust installed via rustup"
}

# Install cargo-leptos
install_cargo_leptos() {
    if ! command -v cargo-leptos &> /dev/null; then
        print_status "Installing cargo-leptos (this may take a while)..."

        if [ "$IS_TERMUX" = true ]; then
            # Low-memory build for Termux
            CARGO_BUILD_JOBS=1 cargo install cargo-leptos --locked
        else
            cargo install cargo-leptos --locked
        fi

        print_success "cargo-leptos installed"
    else
        print_success "cargo-leptos already installed"
    fi
}

# Add wasm target
install_wasm_target() {
    print_status "Adding WASM target..."
    rustup target add wasm32-unknown-unknown
    print_success "WASM target added"
}

# Configure Cargo for low-memory builds (Termux)
configure_cargo_termux() {
    if [ "$IS_TERMUX" = true ]; then
        print_status "Configuring Cargo for low-memory builds..."

        mkdir -p "$HOME/.cargo"

        # Create or update cargo config
        cat > "$HOME/.cargo/config.toml" << 'EOF'
[build]
jobs = 1

[net]
git-fetch-with-cli = true

[term]
verbose = true
EOF

        # Set environment variables for the session
        export CARGO_BUILD_JOBS=1
        export CARGO_INCREMENTAL=1

        print_success "Cargo configured for Termux"
    fi
}

# Clone repository
clone_repo() {
    if [ -d "$INSTALL_DIR" ]; then
        print_status "Updating existing installation..."
        cd "$INSTALL_DIR"
        git pull origin main
    else
        print_status "Cloning repository..."
        git clone "$REPO_URL" "$INSTALL_DIR"
        cd "$INSTALL_DIR"
    fi
    print_success "Repository ready"
}

# Optimize Cargo.toml for Termux
optimize_for_termux() {
    if [ "$IS_TERMUX" = true ]; then
        print_status "Optimizing build settings for Termux..."

        # Check if profile.release already has our optimizations
        if ! grep -q "codegen-units = 1" Cargo.toml; then
            cat >> Cargo.toml << 'EOF'

[profile.release]
codegen-units = 1
lto = false
opt-level = "z"
strip = true

[profile.dev]
opt-level = 0
debug = false
EOF
        fi

        print_success "Build settings optimized"
    fi
}

# Build the project
build_project() {
    print_status "Building project (this will take a while)..."

    cd "$INSTALL_DIR"

    if [ "$IS_TERMUX" = true ]; then
        print_warning "Building on Termux - this may take 15-30+ minutes"
        print_warning "Keep the screen on and Termux in foreground"

        # Acquire wakelock to prevent sleep
        if command -v termux-wake-lock &> /dev/null; then
            termux-wake-lock
            print_status "Wake lock acquired"
        fi

        # Build with minimal resources
        CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=1 cargo leptos build --release 2>&1 | tee build.log

        # Release wakelock
        if command -v termux-wake-unlock &> /dev/null; then
            termux-wake-unlock
        fi
    else
        cargo leptos build --release
    fi

    print_success "Build complete!"
}

# Create run script
create_run_script() {
    print_status "Creating run script..."

    cat > "$INSTALL_DIR/run.sh" << 'EOF'
#!/bin/bash
cd "$(dirname "$0")"
echo "Starting Ollama Rust Web UI on http://localhost:3000"
./target/release/ollama-rust
EOF

    chmod +x "$INSTALL_DIR/run.sh"
    print_success "Run script created"
}

# Main installation
main() {
    install_dependencies
    install_rust

    # Source cargo env
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    configure_cargo_termux
    install_wasm_target
    install_cargo_leptos
    clone_repo
    optimize_for_termux
    build_project
    create_run_script

    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║       Installation Complete!          ║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════╝${NC}"
    echo ""
    echo -e "To start the server:"
    echo -e "  ${BLUE}cd $INSTALL_DIR && ./run.sh${NC}"
    echo ""
    echo -e "Or manually:"
    echo -e "  ${BLUE}cd $INSTALL_DIR${NC}"
    echo -e "  ${BLUE}cargo leptos serve --release${NC}"
    echo ""
    echo -e "Then open ${GREEN}http://localhost:3000${NC} in your browser"
    echo ""
}

# Run main
main
