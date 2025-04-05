#!/bin/bash

# Change to the directory where the script is located
cd "$(dirname "$0")"

# Build the release version
cd ..
echo "Building release version..."
cargo build --release

cd test_data
# Start the nodes in the background
echo "Starting nodes..."
../target/release/match --config config1.toml > match1.log 2>&1 &
NODE1_PID=$!
../target/release/match --config config2.toml > match2.log 2>&1 &
NODE2_PID=$!
../target/release/match --config config3.toml > match3.log 2>&1 &
NODE3_PID=$!

# Wait a bit for nodes to initialize
sleep 3

# Run the benchmark
echo "Starting benchmark..."
../target/release/benchmark --concurrency 100 --interval 1 --duration 30 --server "grpc://127.0.0.1:4001"

# Cleanup: kill the node processes
echo "Cleaning up..."
rm -r data1
rm -r data2
rm -r data3
rm match1.log
rm match2.log
rm match3.log
kill $NODE1_PID $NODE2_PID $NODE3_PID
