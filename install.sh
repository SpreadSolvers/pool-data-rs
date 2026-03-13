BINARY_PATH=./target/release/pool-data-rs
TARGET_PATH=/Users/slava/Development/blockchain/arb/bin

set -e

echo "Building binary"

cargo build --release

echo "Copying binary to $TARGET_PATH"
cp -r $BINARY_PATH $TARGET_PATH/pool-data

echo "Done :D"