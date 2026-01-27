# 1. Get the code if it's not here
if [ ! -d "ollama-rust" ]; then
    echo "Cloning ollama-rust..."
    git clone https://github.com/starlessoblivion/ollama-rust.git
fi

# 2. ALWAYS move into the project directory
# This was likely being skipped or failing because of the logic flow
cd "$HOME/ollama-rust" || { echo "Failed to find ollama-rust directory"; exit 1; }

# 3. Setup Environment Variables
export OPENSSL_DIR=$PREFIX
export LDFLAGS="-L$PREFIX/lib"
export CPPFLAGS="-I$PREFIX/include"

echo "Building project (this will take a minute)..."
# Now cargo will find the Cargo.toml file
cargo build --release
