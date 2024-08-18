#!/bin/bash

# Run the client to interact with nodes
for i in {1..5}
do
    PORT=$((8000 + i))
    NODE_ID="node$i"
    NODE_ID=$NODE_ID cargo run --release --bin client
done
