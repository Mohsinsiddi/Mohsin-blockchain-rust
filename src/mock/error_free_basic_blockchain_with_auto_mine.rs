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
use std::time::{Duration, Instant};
use tokio::time::sleep;
use rand::distributions::Alphanumeric;
use rand::Rng as _; // Use `rand::Rng` for randomness
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String, // Unique ID for transaction
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
    pub block_hash: Option<String>, // Reference to the block this transaction is in
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub proof: u64,
    pub hash: String,
}

#[derive(Clone)]
pub struct Blockchain {
    db: Db,
    token_balances: Arc<Mutex<HashMap<String, u64>>>, // Track token balances by address
    transactions: Arc<Mutex<HashMap<String, Transaction>>>, // Track transactions by ID
    current_block: Arc<Mutex<Option<Block>>>, // Current block being mined
}

impl Blockchain {
    pub fn new(path: &str) -> Self {
        let db = sled::open(path).expect("Failed to open database");
        Blockchain {
            db,
            token_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(HashMap::new())),
            current_block: Arc::new(Mutex::new(None)),
        }
    }

    pub fn add_block(&self, block: Block) {
        let key = block.index.to_string();
        let value = serde_json::to_vec(&block).unwrap();
        self.db.insert(key, value).expect("Failed to write block to database");
        info!("Block {} mined and added to the blockchain.", block.index);

        // Store the current block as None once a block is added
        let mut current_block = self.current_block.lock().unwrap();
        *current_block = None;
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

    pub fn add_transaction(&self, transaction: Transaction) {
        let mut txs = self.transactions.lock().unwrap();
        txs.insert(transaction.id.clone(), transaction.clone());
    }

    pub fn get_transaction(&self, id: &str) -> Option<Transaction> {
        let txs = self.transactions.lock().unwrap();
        txs.get(id).cloned()
    }

    pub fn start_mining(&self, pos: Arc<PoS>) {
        let blockchain = self.clone();
        tokio::spawn(async move {
            loop {
                let block = blockchain.mine_block(&pos).await;
                blockchain.add_block(block);
                sleep(Duration::from_secs(2)).await; // Mining interval
            }
        });
    }

    async fn mine_block(&self, pos: &Arc<PoS>) -> Block {
        let mut current_block = self.current_block.lock().unwrap();
        let block_index = match *current_block {
            Some(ref block) => block.index + 1,
            None => 1, // First block
        };
        let previous_hash = match *current_block {
            Some(ref block) => block.hash.clone(),
            None => "0".to_string(), // Genesis block
        };

        let mut transactions = self.transactions.lock().unwrap();
        let tx_list: Vec<Transaction> = transactions.values().cloned().collect();
        let transactions_to_include: Vec<Transaction> = tx_list.into_iter().take(10).collect(); // Limit block size

        let block = Block {
            index: block_index,
            previous_hash: previous_hash.clone(),
            timestamp: Utc::now().timestamp() as u64,
            transactions: transactions_to_include.clone(),
            proof: Self::proof_of_work(&previous_hash),
            hash: Self::calculate_hash(block_index, &previous_hash, &transactions_to_include, Self::proof_of_work(&previous_hash)),
        };

        for transaction in block.transactions.iter() {
            let mut txs = self.transactions.lock().unwrap();
            if let Some(mut tx) = txs.remove(&transaction.id) {
                tx.block_hash = Some(block.hash.clone());
                txs.insert(transaction.id.clone(), tx);
            }
        }

        *current_block = Some(block.clone());
        block
    }

    fn proof_of_work(previous_hash: &str) -> u64 {
        let mut proof = 0;
        let target = "0000"; // Simplified target for proof of work

        while !Self::calculate_hash(0, previous_hash, &[], proof).starts_with(target) {
            proof += 1;
        }

        proof
    }

    fn calculate_hash(index: u64, previous_hash: &str, transactions: &[Transaction], proof: u64) -> String {
        let transactions_str = transactions.iter().map(|t| format!("{:?}", t)).collect::<String>();
        let input = format!("{}{}{}{}{}", index, previous_hash, transactions_str, proof, "MOHSIN");
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = hasher.finalize();
        hex::encode(result)
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

// Function to generate a new key pair
fn generate_key_pair() -> KeyPair {
    let private_key: String = rand::thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let public_key = Sha256::digest(private_key.as_bytes());
    KeyPair {
        private_key,
        public_key: hex::encode(public_key),
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
    let validator_addresses: Vec<String> = (0..5).map(|_| generate_key_pair().public_key).collect();
    for address in validator_addresses {
        pos.add_validator(address, 100); // Assign some stake
    }

    blockchain.start_mining(pos.clone()); // Start mining in a background task

    let blockchain_filter = warp::any().map(move || blockchain.clone());
    let pos_filter = warp::any().map(move || pos.clone());

    let new_address = warp::path("new_address")
        .and(warp::get())
        .map(|| {
            let key_pair = generate_key_pair();
            warp::reply::json(&key_pair)
        });

    let validate_key = warp::path("validate_key")
        .and(warp::get())
        .and(warp::query::<HashMap<String, String>>())
        .map(|query: HashMap<String, String>| {
            let binding = "".to_string();
            let private_key = query.get("private_key").unwrap_or(&binding);
            let binding = "".to_string();
            let public_key = query.get("public_key").unwrap_or(&binding);
            let valid = validate_key(private_key, public_key);
            warp::reply::json(&format!("Key pair is valid: {}", valid))
        });

    let balance = warp::path("balance")
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(blockchain_filter.clone())
        .and_then(get_balance);

    let transaction = warp::path("transaction")
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(blockchain_filter.clone())
        .and_then(get_transaction);

        let transactions = warp::path("transactions")
        .and(warp::post())
        .and(warp::body::json())
        .and(blockchain_filter.clone())
        .and(pos_filter.clone()) // Include PoS filter
        .and_then(|transaction: Transaction, blockchain, pos| handle_transaction(transaction, blockchain, pos));


    let routes = new_address
        .or(validate_key)
        .or(balance)
        .or(transaction)
        .or(transactions);

    println!("Starting server on port 3030...");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn handle_transaction(
    transaction: Transaction,
    blockchain: Arc<Blockchain>,
    pos: Arc<PoS>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut blockchain = blockchain.clone();
    let mut current_block = blockchain.current_block.lock().unwrap();

    let mut new_block;
    if let Some(block) = &*current_block {
        let mut new_block_data = block.clone();
        new_block_data.transactions.push(transaction.clone());
        new_block_data.hash = Blockchain::calculate_hash(
            new_block_data.index,
            &new_block_data.previous_hash,
            &new_block_data.transactions,
            new_block_data.proof,
        );
        new_block = new_block_data;
    } else {
        let transactions = vec![transaction.clone()];
        new_block = Block {
            index: 1,
            previous_hash: "0".to_string(),
            timestamp: Utc::now().timestamp() as u64,
            transactions: transactions.clone(),
            proof: Blockchain::proof_of_work("0"),
            hash: Blockchain::calculate_hash(
                1,
                "0",
                &transactions,
                Blockchain::proof_of_work("0"),
            ),
        };
    }

    blockchain.add_block(new_block);

    blockchain.add_transaction(transaction.clone());

    let validator = pos.select_validator();
    if let Some(validator) = validator {
        let reward = 10; // Simplified reward
        blockchain.update_balance(&validator.address, reward);
        info!("Reward of {} MOHSIN tokens distributed to validator at address {}.", reward, validator.address);
    } else {
        info!("No validator selected for reward distribution.");
    }

    Ok(warp::reply::json(&format!("Transaction added with ID: {}", transaction.id)))
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
