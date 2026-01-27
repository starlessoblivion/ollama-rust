#!/bin/bash
# Ollama-Rust Android Installer

echo "Installing dependencies..."
pkg update -y && pkg upgrade -y
pkg install -y git rust binutils build-essential openssl openssl-tool

# 1. Clone the repo if we aren't already in it
if [ ! -d "ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git
fi

cd ollama-rust || exit

# 2. Setup Environment Variables
export OPENSSL_DIR=$PREFIX
export LDFLAGS="-L$PREFIX/lib"
export CPPFLAGS="-I$PREFIX/include"

echo "Building project..."
cargo build --release

# 3. Create a permanent shortcut in the home directory
echo "#!/bin/bash
export OPENSSL_DIR=$PREFIX
export LDFLAGS=\"-L$PREFIX/lib\"
export CPPFLAGS=\"-I$PREFIX/include\"
cd $(pwd)
./target/release/ollama-rust" > ~/run-ollama.sh

chmod +x ~/run-ollama.sh

echo "-------------------------------------------"
echo "Done! From now on, just type: ./run-ollama.sh"
echo "-------------------------------------------"
