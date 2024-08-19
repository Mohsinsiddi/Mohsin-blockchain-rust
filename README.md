````markdown
# Mohsin Blockchain

Welcome to the Mohsin Blockchain project! This project is a simple blockchain implementation written in Rust.

## Prerequisites

Before running the blockchain, ensure you have Rust and Cargo installed. You can install them by following these steps:

### Install Rust and Cargo

1. **Install Rust**:

   - Go to the [official Rust website](https://www.rust-lang.org/).
   - Follow the instructions to install Rust using `rustup`.

   Alternatively, you can run the following command in your terminal:

   ```sh
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
````

2. **Verify Installation**:

   - Ensure Rust and Cargo are installed correctly by checking their versions:

     ```sh
     rustc --version
     cargo --version
     ```

## Running the Blockchain

1. **Clone the Repository**:

   ```sh
   git clone https://github.com/Mohsinsiddi/Mohsin-blockchain-rust.git
   cd Mohsin-blockchain-rust
   ```

2. **Build and Run the Blockchain**:

   ```sh
   cargo clean && cargo build
   RUST_LOG=info cargo run
   ```

   This will start the blockchain server on port 3030.

<img width="1512" alt="Screenshot 2024-08-19 at 2 39 04 PM" src="https://github.com/user-attachments/assets/2fadbfce-45cb-49ee-942d-d277d770249b">
   
## Testing the Blockchain

You can use `curl` to interact with the blockchain. Below are the commands to test various features.

### 1. Generate a New Address

```sh
curl -X GET http://localhost:3030/new_address
```

### 2. Airdrop MOHSIN Tokens to the New Address

**Note:** Before testing the transfer tokens feature, you must airdrop tokens to the newly created address to ensure it has a balance.

Replace `ADDRESS` with the address you received from the previous step:

```sh
curl -X POST http://localhost:3030/airdrop_tokens \
    -H "Content-Type: application/json" \
    -d '{"address": "ADDRESS", "amount": 1000}'
```
<img width="1019" alt="Screenshot 2024-08-19 at 2 47 36 PM" src="https://github.com/user-attachments/assets/2c24b8c0-6e71-47c0-986e-347250cb5072">

### 3. Check Balance of an Address

Replace `ADDRESS` with the actual address you want to check.

```sh
curl -X GET http://localhost:3030/balance/ADDRESS
```
<img width="1019" alt="Screenshot 2024-08-19 at 2 40 01 PM" src="https://github.com/user-attachments/assets/b894069e-e20a-4a68-8f9e-c62784916d89">

### 4. Add a Transaction

Replace `SENDER_ADDRESS`, `RECIPIENT_ADDRESS`, and `AMOUNT` with the relevant values.

```sh
curl -X POST http://localhost:3030/transaction \
    -H "Content-Type: application/json" \
    -d '{"id": "TX_ID", "sender": "SENDER_ADDRESS", "recipient": "RECIPIENT_ADDRESS", "amount": AMOUNT}'
```

### 5. Transfer Tokens

Replace `FROM_ADDRESS`, `TO_ADDRESS`, and `AMOUNT` with the relevant values. Ensure `FROM_ADDRESS` has sufficient tokens (including the fee) by following the previous airdrop step.

```sh
curl -X POST http://localhost:3030/transfer \
    -H "Content-Type: application/json" \
    -d '{"from": "FROM_ADDRESS", "to": "TO_ADDRESS", "amount": AMOUNT}'
```
<img width="1001" alt="Screenshot 2024-08-19 at 2 39 35 PM" src="https://github.com/user-attachments/assets/772bd441-f291-4ed0-ba8a-004529aad191">

### 6. Get Transaction Details by ID

Replace `TRANSACTION_ID` with the ID of the transaction you want to retrieve.

```sh
curl -X GET http://localhost:3030/transaction/TRANSACTION_ID
```

## Project Structure

- `src/main.rs` - Contains the blockchain implementation and Warp server setup.
- `Cargo.toml` - Contains project dependencies and metadata.

## Contributing

If you would like to contribute to this project, please fork the repository and submit a pull request with your changes.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contact

For any questions, feel free to reach out to [Mohsin Siddi](https://github.com/Mohsinsiddi).

```

This updated README now includes the new feature for airdropping tokens and stresses the importance of performing the airdrop before testing token transfers.
```
