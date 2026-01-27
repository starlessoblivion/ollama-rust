#!/bin/bash
# Ollama-Rust Android Installer

echo "Installing dependencies..."
pkg update -y && pkg upgrade -y
pkg install -y git rust binutils build-essential openssl openssl-tool

# Setup Environment Variables (The 'crap' you want to automate)
export OPENSSL_DIR=$PREFIX
export LDFLAGS="-L$PREFIX/lib"
export CPPFLAGS="-I$PREFIX/include"

echo "Building project..."
cargo build --release

# Create a permanent shortcut so you don't have to export paths manually again
echo "#!/bin/bash
export OPENSSL_DIR=$PREFIX
export LDFLAGS=\"-L$PREFIX/lib\"
export CPPFLAGS=\"-I$PREFIX/include\"
$(pwd)/target/release/ollama-rust" > ~/run-ollama.sh

chmod +x ~/run-ollama.sh

echo "-------------------------------------------"
echo "Done! From now on, just type: ./run-ollama.sh"
echo "-------------------------------------------"
