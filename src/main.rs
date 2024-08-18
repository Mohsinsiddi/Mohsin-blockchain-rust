use std::collections::{HashMap, HashSet};
use rand::{seq::IteratorRandom, Rng};
use sha2::{Sha256, Digest};
use hex::encode;
use serde::{Serialize, Deserialize};
use warp::Filter;
use tokio;
use chrono::Utc;
use std::sync::{Arc, Mutex};
use log::{info, error, debug};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub amount: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u64,
    pub previous_hash: String,
    pub timestamp: u64,
    pub transaction: Option<Transaction>,
    pub proof: u64,
    pub hash: String,
    pub validator: String, // New field to store the validator
}

#[derive(Clone)]
pub struct Blockchain {
    db: Arc<Mutex<HashMap<u64, Block>>>,
    token_balances: Arc<Mutex<HashMap<String, u64>>>,
    transactions: Arc<Mutex<HashMap<String, Transaction>>>,
    current_block: Arc<Mutex<Option<Block>>>,
    block_index: Arc<Mutex<u64>>,
    mempool: Arc<Mutex<Vec<Transaction>>>,
    airdropped_addresses: Arc<Mutex<HashSet<String>>>,
    validators: Arc<Mutex<HashSet<String>>>, // Set of validators
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            db: Arc::new(Mutex::new(HashMap::new())),
            token_balances: Arc::new(Mutex::new(HashMap::new())),
            transactions: Arc::new(Mutex::new(HashMap::new())),
            current_block: Arc::new(Mutex::new(None)),
            block_index: Arc::new(Mutex::new(1)),
            mempool: Arc::new(Mutex::new(Vec::new())),
            airdropped_addresses: Arc::new(Mutex::new(HashSet::new())),
            validators: Arc::new(Mutex::new(HashSet::new())),
        };

        // Create and airdrop tokens to a random address at startup
        blockchain.airdrop_tokens(1000); // Airdrop 1000 tokens

        // Add 5 validators
        for _ in 0..5 {
            blockchain.add_validator(Self::generate_random_address());
        }

        blockchain
    }

    pub fn get_transaction(&self, id: &str) -> Option<Transaction> {
        let txs = self.transactions.lock().unwrap();
        txs.get(id).cloned()
    }

    pub fn get_last_block(&self) -> Option<Block> {
        let last_index = {
            let index = self.block_index.lock().unwrap();
            *index - 1
        };
        let db = self.db.lock().unwrap();
        db.get(&last_index).cloned()
    }

    pub fn add_block(&self, block: Block) {
        let mut db = self.db.lock().unwrap();
        db.insert(block.index, block.clone());

        // Update the current block to None after adding it
        let mut current_block = self.current_block.lock().unwrap();
        *current_block = None;

        info!("Block added with index: {}, hash: {}, validator: {}", block.index, block.hash, block.validator);

        // Reward the validator
        self.update_balance(&block.validator, 1).unwrap();
    }

    pub fn get_block(&self, index: u64) -> Option<Block> {
        let db = self.db.lock().unwrap();
        db.get(&index).cloned()
    }

    pub fn update_balance(&self, address: &str, amount: i64) -> Result<(), &'static str> {
        let mut balances = self.token_balances.lock().unwrap();
        let balance = balances.entry(address.to_string()).or_insert(0);

        // Check if balance is sufficient for withdrawal
        if *balance as i64 + amount < 0 {
            return Err("Insufficient funds");
        }

        *balance = (*balance as i64 + amount) as u64; // Ensure no negative balances
        Ok(())
    }

    pub fn get_balance(&self, address: &str) -> u64 {
        let balances = self.token_balances.lock().unwrap();
        *balances.get(address).unwrap_or(&0)
    }

    pub fn add_transaction(&self, transaction: Transaction) {
        {
            let mut txs = self.transactions.lock().unwrap();
            txs.insert(transaction.id.clone(), transaction.clone());
            debug!("Transaction added: {:?}", transaction); // Log added transaction
        }
    
        {
            let mut mempool = self.mempool.lock().unwrap();
            mempool.push(transaction);
        }
    }

    pub fn start_mining(&self) {
        let blockchain = self.clone();
        tokio::spawn(async move {
            loop {
                let block = blockchain.mine_block().await;
                blockchain.add_block(block);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        });
    }

    async fn mine_block(&self) -> Block {
        let block_index = {
            let mut index = self.block_index.lock().unwrap();
            let current_index = *index;
            *index += 1;
            current_index
        };

        let previous_hash = match *self.current_block.lock().unwrap() {
            Some(ref block) => block.hash.clone(),
            None => "0".to_string(),
        };

        let mut mempool = self.mempool.lock().unwrap();
        let transaction_to_include = mempool.pop(); // Take one transaction

        let proof = Self::proof_of_work(&previous_hash);
        let hash = Self::calculate_hash(block_index, &previous_hash, &[transaction_to_include.clone()], proof);

        // Select a validator (randomly for simplicity)
        let validator = {
            let validators = self.validators.lock().unwrap();
            validators.iter().cloned().choose(&mut rand::thread_rng()).unwrap_or_else(|| "None".to_string())
        };

        let block = Block {
            index: block_index,
            previous_hash: previous_hash.clone(),
            timestamp: Utc::now().timestamp() as u64,
            transaction: transaction_to_include.clone(),
            proof,
            hash,
            validator,
        };

        // Set the new block as the current block
        let mut current_block = self.current_block.lock().unwrap();
        *current_block = Some(block.clone());

        // If no transaction was included, airdrop tokens to a random validator
        if transaction_to_include.is_none() {
            self.airdrop_tokens_to_random_validator(2); // Airdrop 100 tokens
        }

        block
    }

    fn proof_of_work(previous_hash: &str) -> u64 {
        let mut proof = 0;
        let target = "0000";
        while !Self::calculate_hash(0, previous_hash, &[], proof).starts_with(target) {
            proof += 1;
        }
        proof
    }

    fn calculate_hash(index: u64, previous_hash: &str, transaction: &[Option<Transaction>], proof: u64) -> String {
        let transaction_str = transaction.iter().map(|t| format!("{:?}", t)).collect::<String>();
        let input = format!("{}{}{}{}{}", index, previous_hash, transaction_str, proof, "MOHSIN");
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = hasher.finalize();
        encode(result)
    }

    pub fn add_validator(&self, address: String) {
        let mut validators = self.validators.lock().unwrap();
        validators.insert(address.clone());
        info!("Validator added: {}", address);
    }

    pub fn remove_validator(&self, address: &str) {
        let mut validators = self.validators.lock().unwrap();
        if validators.remove(address) {
            info!("Validator removed: {}", address);
        } else {
            error!("Validator {} not found", address);
        }
    }

    pub fn airdrop_tokens(&self, amount: u64) {
        let address = Self::generate_random_address();
        let mut airdropped_addresses = self.airdropped_addresses.lock().unwrap();
        if !airdropped_addresses.contains(&address) {
            self.update_balance(&address, amount as i64).unwrap();
            airdropped_addresses.insert(address.clone());
            info!("Airdropped {} MOHSIN tokens to address {}", amount, address);
        } else {
            error!("Address {} already airdropped", address);
        }
    }

    fn airdrop_tokens_to_random_validator(&self, amount: u64) {
        let validator = {
            let validators = self.validators.lock().unwrap();
            validators.iter().cloned().choose(&mut rand::thread_rng()).unwrap_or_else(|| "None".to_string())
        };
        
        if validator != "None" {
            self.update_balance(&validator, amount as i64).unwrap();
            info!("Airdropped {} MOHSIN tokens to validator {}", amount, validator);
        } else {
            error!("No validators available for airdrop");
        }
    }

    fn generate_random_address() -> String {
        let mut rng = rand::thread_rng();
        (0..64).map(|_| rng.sample(rand::distributions::Alphanumeric) as char).collect()
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let blockchain = Arc::new(Blockchain::new());
    blockchain.start_mining(); // Start mining in a background task

    let blockchain_filter = warp::any().map(move || blockchain.clone());

    let new_address = warp::path("new_address")
        .and(warp::get())
        .map(|| {
            let key_pair = generate_key_pair();
            warp::reply::json(&key_pair)
        });

    let balance = warp::path("balance")
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(blockchain_filter.clone())
        .map(|address: String, blockchain: Arc<Blockchain>| {
            let balance = blockchain.get_balance(&address);
            warp::reply::json(&format!("Balance for address {}: {} MOHSIN tokens", address, balance))
        });

    let transaction = warp::path("transaction")
        .and(warp::post())
        .and(warp::body::json())
        .and(blockchain_filter.clone())
        .map(|transaction: Transaction, blockchain: Arc<Blockchain>| {
            blockchain.add_transaction(transaction.clone());
            warp::reply::json(&format!("Transaction added with ID: {}", transaction.id))
        });

    let transfer_tokens = warp::path("transfer")
        .and(warp::post())
        .and(warp::body::json())
        .and(blockchain_filter.clone())
        .map(|transfer: TransferRequest, blockchain: Arc<Blockchain>| {
            let TransferRequest { from, to, amount } = transfer;
            if blockchain.update_balance(&from, -(amount as i64 + 1)).is_err() { // Deduct fee here
                return warp::reply::json(&"Insufficient funds".to_string());
            }
            blockchain.update_balance(&to, amount as i64).unwrap();
            let transaction = Transaction {
                id: generate_transaction_id(),
                sender: from.clone(),
                recipient: to.clone(),
                amount,
            };
            blockchain.add_transaction(transaction.clone());
            // Reward a randomly selected validator with the fee
            let validator = {
                let validators = blockchain.validators.lock().unwrap();
                validators.iter().cloned().choose(&mut rand::thread_rng()).unwrap_or_else(|| "None".to_string())
            };
            if validator != "None" {
                blockchain.update_balance(&validator, 1).unwrap();
            }
            warp::reply::json(&format!("Transferred {} MOHSIN tokens from {} to {}. Transaction ID: {}", amount, from, to, transaction.id))
        });


    let transaction_details = warp::path("transaction")
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(blockchain_filter.clone())
        .and_then(get_transaction);
    
    let routes = new_address
        .or(balance)
        .or(transaction)
        .or(transaction_details)
        .or(transfer_tokens);
    
    println!("Starting server on port 3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[derive(Deserialize)]
struct TransferRequest {
    from: String,
    to: String,
    amount: u64,
}

fn generate_key_pair() -> KeyPair {
    let private_key = rand::thread_rng().gen::<[u8; 32]>();
    let public_key = Sha256::digest(&private_key);
    KeyPair {
        private_key: encode(private_key),
        public_key: encode(public_key),
    }
}

fn generate_transaction_id() -> String {
    let mut rng = rand::thread_rng();
    (0..8).map(|_| rng.sample(rand::distributions::Alphanumeric) as char).collect()
}

async fn get_transaction(id: String, blockchain: Arc<Blockchain>) -> Result<impl warp::Reply, warp::Rejection> {
    match blockchain.get_transaction(&id) {
        Some(transaction) => Ok(warp::reply::json(&transaction)),
        None => Ok(warp::reply::json(&format!("Transaction with ID {} not found", id))),
    }
}
