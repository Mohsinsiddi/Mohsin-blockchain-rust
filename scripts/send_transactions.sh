#!/bin/bash

# Send a transaction to the first node
curl -X POST http://localhost:3030/transactions -d '{"sender":"Alice","recipient":"Bob","amount":50}'

# You can add similar curl commands for other nodes if needed
