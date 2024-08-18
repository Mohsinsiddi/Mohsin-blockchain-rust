use serde::{Deserialize, Serialize};
use sled::Db;
use rand::Rng;
use warp::Filter;
use std::sync::{Arc, Mutex};
use warp::http::StatusCode;
use log::{info, error};
use tokio;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use rand::distributions::Alphanumeric;
use rand::Rng as _;

// Structure for Key Pair
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

// Function to generate a new key pair
fn generate_key_pair() -> KeyPair {
    let private_key: String = rand::thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let public_key = Sha256::digest(private_key.as_bytes());
    KeyPair {
        private_key,
        public_key: hex::encode(public_key),
    }
}

// Structure for Transaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
}

// Structure for Block
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub proof: u64,
}

// Structure for Blockchain
#[derive(Clone)]
pub struct Blockchain {
    db: Db,
    token_balances: Arc<Mutex<HashMap<String, u64>>>, // Track token balances by address
    transactions: Arc<Mutex<HashMap<String, Transaction>>>, // Track transactions by ID
}

impl Blockchain {
    pub fn new(path: &str) -> Self {
        let db = sled::open(path).expect("Failed to open database");
        Blockchain {
            db,
            token_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_block(&self, block: Block) {
        let key = block.index.to_string();
        let value = serde_json::to_vec(&block).unwrap();
        self.db.insert(key, value).expect("Failed to write block to database");
        info!("Block {} mined and added to the blockchain.", block.index);
    }

    pub fn get_block(&self, index: u64) -> Option<Block> {
        let key = index.to_string();
        if let Some(value) = self.db.get(key).expect("Failed to get block from database") {
            return serde_json::from_slice(&value).ok();
        }
        None
    }

    pub fn update_balance(&self, address: &str, amount: u64) {
        let mut balances = self.token_balances.lock().unwrap();
        let balance = balances.entry(address.to_string()).or_insert(0);
        *balance += amount;
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        let balances = self.token_balances.lock().unwrap();
        *balances.get(address).unwrap_or(&0)
    }

    pub fn add_transaction(&self, id: &str, transaction: Transaction) {
        let mut txs = self.transactions.lock().unwrap();
        txs.insert(id.to_string(), transaction);
    }

    pub fn get_transaction(&self, id: &str) -> Option<Transaction> {
        let txs = self.transactions.lock().unwrap();
        txs.get(id).cloned()
    }
}

#[derive(Debug, Clone)]
pub struct Validator {
    pub address: String,
    pub stake: u64,
}

#[derive(Clone)]
pub struct PoS {
    validators: Arc<Mutex<Vec<Validator>>>,
}

impl PoS {
    pub fn new() -> Self {
        PoS {
            validators: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_validator(&self, address: String, stake: u64) {
        let mut validators = self.validators.lock().unwrap();
        validators.push(Validator { address, stake });
    }

    pub fn select_validator(&self) -> Option<Validator> {
        let validators = self.validators.lock().unwrap();
        if validators.is_empty() {
            return None;
        }
        let total_stake: u64 = validators.iter().map(|v| v.stake).sum();
        let mut rng = rand::thread_rng();
        let rand_value = rng.gen_range(0..total_stake);
        let mut cumulative_stake = 0;
        for validator in validators.iter() {
            cumulative_stake += validator.stake;
            if rand_value < cumulative_stake {
                return Some(validator.clone());
            }
        }
        None
    }
}

// Function to validate public key
fn validate_key(private_key: &str, public_key: &str) -> bool {
    let expected_public_key = Sha256::digest(private_key.as_bytes());
    hex::encode(expected_public_key) == public_key
}

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init();

    let blockchain = Arc::new(Blockchain::new("mydb"));
    let pos = Arc::new(PoS::new());

    // Initialize example validators
    let validator_addresses: Vec<String> = (0..3)
        .map(|_| generate_key_pair().public_key)
        .collect();
    
    let pos_clone = pos.clone();
    for address in validator_addresses {
        pos_clone.add_validator(address.clone(), 1000);
        blockchain.update_balance(&address, 100); // Airdrop 100 MOHSIN tokens
        info!("Airdropped 100 MOHSIN tokens to address {}", address);
    }

    let blockchain_filter = warp::any().map(move || blockchain.clone());
    let pos_filter = warp::any().map(move || pos.clone());

    // Endpoint to handle transactions
    let transactions = warp::path("transactions")
        .and(warp::post())
        .and(warp::body::json())
        .and(blockchain_filter.clone())
        .and(pos_filter.clone())
        .and_then(handle_transaction);

    // Endpoint to get balance
    let balance = warp::path!("balance" / String)
        .and(warp::get())
        .and(blockchain_filter.clone())
        .and_then(get_balance);

    // Endpoint to generate new address
    let new_address = warp::path("new_address")
        .and(warp::get())
        .map(|| {
            let key_pair = generate_key_pair();
            format!(
                "Private Key: {}\nPublic Key: {}",
                key_pair.private_key, key_pair.public_key
            )
        });

    // Endpoint to validate a key pair
    let validate_key = warp::path("validate_key")
        .and(warp::get())
        .and(warp::query::<KeyPair>())
        .map(|key_pair: KeyPair| {
            let is_valid = validate_key(&key_pair.private_key, &key_pair.public_key);
            if is_valid {
                format!("The public key is valid for the provided private key.")
            } else {
                format!("The public key is NOT valid for the provided private key.")
            }
        });

    // Endpoint to get transaction by ID
    let transaction = warp::path!("transaction" / String)
        .and(warp::get())
        .and(blockchain_filter.clone())
        .and_then(get_transaction);

    let routes = transactions.or(balance).or(new_address).or(validate_key).or(transaction);

    println!("Starting server on port 3030...");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_transaction(
    transaction: Transaction,
    blockchain: Arc<Blockchain>,
    pos: Arc<PoS>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // Generate a unique transaction ID (for simplicity, use a hash of the transaction details)
    let transaction_id = Sha256::digest(format!("{:?}", transaction).as_bytes());
    let transaction_id_hex = hex::encode(transaction_id);

    blockchain.add_transaction(&transaction_id_hex, transaction.clone());

    let block = Block {
        index: 1, // Simplified; in a real scenario, use a proper index.
        previous_hash: "0".to_string(),
        timestamp: 1234567890,
        transactions: vec![transaction],
        proof: 100, // Simplified; in a real scenario, use a proper proof.
    };

    blockchain.add_block(block);

    // Log reward distribution
    if let Some(validator) = pos.select_validator() {
        let reward = 10; // Simplified reward
        blockchain.update_balance(&validator.address, reward);
        info!("Reward of {} MOHSIN tokens distributed to validator at address {}.", reward, validator.address);
    } else {
        info!("No validator selected for reward distribution.");
    }

    Ok(warp::reply::with_status(format!("Transaction added with ID: {}", transaction_id_hex), StatusCode::OK))
}

async fn get_balance(address: String, blockchain: Arc<Blockchain>) -> Result<impl warp::Reply, warp::Rejection> {
    let balance = blockchain.get_balance(&address);
    Ok(warp::reply::json(&format!("Balance for address {}: {} MOHSIN tokens", address, balance)))
}

async fn get_transaction(id: String, blockchain: Arc<Blockchain>) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(transaction) = blockchain.get_transaction(&id) {
        Ok(warp::reply::json(&transaction))
    } else {
        Ok(warp::reply::json(&format!("Transaction not found")))
    }
}
