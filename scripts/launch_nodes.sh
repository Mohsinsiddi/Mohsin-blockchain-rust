#!/bin/bash

# Start the first node
cargo run --bin node -- --port 3030 &

# Start the second node
cargo run --bin node -- --port 3031 &

# Start the third node
cargo run --bin node -- --port 3032 &
