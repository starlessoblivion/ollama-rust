#!/bin/bash
# Ollama-Rust Android Installer

echo "Installing dependencies..."
pkg update -y && pkg upgrade -y
pkg install -y git rust binutils build-essential openssl openssl-tool

# 1. Get the code if it's not here
if [ ! -d "ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git
fi

# 2. Move into the project directory
cd ollama-rust || { echo "Failed to enter directory"; exit 1; }

# 3. Setup Environment Variables
export OPENSSL_DIR=$PREFIX
export LDFLAGS="-L$PREFIX/lib"
export CPPFLAGS="-I$PREFIX/include"

echo "Building project (this will take a minute)..."
cargo build --release

# 4. Create the shortcut in the home directory
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
