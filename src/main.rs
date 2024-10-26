use std::{sync::LazyLock, vec};

#[cfg(feature = "benchmark")]
use std::time::Instant;

#[cfg(not(feature = "reproduce_blocks"))]
use chrono::prelude::*;
use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlag, RandomXVM};
use sha2::{Digest, Sha256};

const BALANCES: [(&str, u8); 2] = [("Master", 150), ("Alice", 20)];

const HASHES: [&str; 8] = [
    "00000000000000000000ecfcf0073a9ae7fd9149d643fa462109f5b0777f5720",
    "00000000000000000001924bab37e9d87715e84aa7bcd0b52405f893dfe7005f",
    "000000000000000000031e67755d995d78beeb40ec0f4f0572b2a94a2cc5c6be",
    "000000000000000000006898028bbd4e6e86d4e1613899353e7f61baebaacd47",
    "00000000000000000001ec97d16e29a1aa5ae14dd4b5103098a0ec0ab7d6f407",
    "0000000000000000000309eb0182d37508dcf8addde651be2f120037697151fd",
    "000000000000000000004d8859f7cacd8834a8e0db41808307242593c746badb",
    "0000000000000000000055e6c36555475a4bf88e62e34b71d4a677b8b0ea64aa",
];

const VM: LazyLock<RandomXVM> = LazyLock::new(|| {
    let now = Instant::now();
    let flags = RandomXFlag::get_recommended_flags() | RandomXFlag::FLAG_FULL_MEM;
    let key = "Key";
    let cache = RandomXCache::new(flags, key.as_bytes()).unwrap();
    println!("VMini: Cache created in {:?}", now.elapsed());
    let dataset = RandomXDataset::new(flags, cache.clone(), 0).unwrap();
    println!("VMini: Dataset created in {:?}", now.elapsed());
    let vm = RandomXVM::new(flags, Some(cache), Some(dataset)).unwrap();
    println!("VMini: Time taken: {:?}", now.elapsed());
    vm
});

#[derive(Debug)]
pub struct Trasaction {
    pub timestamp: u128,
    pub from: String,
    pub to: String,
    pub value: u128,
    pub data: String,
}

impl Trasaction {
    pub fn to_str(&self) -> String {
        let mut data = String::new();
        data.push_str(&format!("{}", self.timestamp));
        data.push(':');
        data.push_str(&self.from);
        data.push(':');
        data.push_str(&self.to);
        data.push(':');
        data.push_str(&format!("{}", self.value));
        data.push(':');
        data.push_str(&self.data);
        data.push(':');
        data.push_str(&self.hash());
        data.push(';');
        data
    }

    fn hash(&self) -> String {
        let input = format!(
            "{}:{}:{}:{}:{}",
            self.timestamp, self.from, self.to, self.value, self.value
        );
        let mut hasher = Sha256::new();
        hasher.update(input);
        let result = hasher.finalize();
        format!("{:x}", result)
    }
}

#[derive(Debug)]
pub struct Block {
    pub index: u32,
    pub timestamp: String,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub btc_hash: String,
    pub difficulty: u8,
}

impl Block {
    fn new(
        index: u32,
        data: String,
        previous_hash: String,
        btc_hash: String,
        difficulty: u8,
        vm: &RandomXVM,
    ) -> Block {
        #[cfg(not(feature = "reproduce_blocks"))]
        let timestamp = Utc::now().to_string();
        #[cfg(feature = "reproduce_blocks")]
        let timestamp = index.to_string();
        let hash = Block::calculate_hash(
            index,
            &timestamp,
            &data,
            &previous_hash,
            &btc_hash,
            difficulty,
            vm,
        );

        Block {
            index,
            timestamp,
            data,
            previous_hash,
            hash,
            btc_hash,
            difficulty,
        }
    }

    fn calculate_hash(
        index: u32,
        timestamp: &str,
        data: &str,
        previous_hash: &str,
        btc_hash: &str,
        difficulty: u8,
        vm: &RandomXVM,
    ) -> String {
        #[cfg(feature = "benchmark")]
        let start = Instant::now();
        let hash: String;
        let mut nonce = 0;
        let i = btc_hash.len() - difficulty as usize;
        let trailing = &btc_hash[i..];
        println!("Trailing: {}", trailing);
        loop {
            let input = format!("{}{}{}{}{}", index, timestamp, data, previous_hash, nonce);
            let h = if cfg!(feature = "randomx") {
                let hash = vm.calculate_hash(input.as_bytes()).expect("no data");
                let hash_str = hash
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>();
                hash_str
            } else {
                let mut hasher = Sha256::new();
                hasher.update(input);
                let result = hasher.finalize();
                format!("{:x}", result)
            };
            let _hash = h;
            if _hash.ends_with(trailing) {
                hash = _hash;
                println!("{}", nonce);
                break;
            } else {
                nonce += 1;
            }
        }
        #[cfg(feature = "benchmark")]
        {
            let duration = start.elapsed();
            //Calculate Hash Rate with nonce and duration
            let hash_rate = nonce as f64 / duration.as_secs_f64();
            println!("Time taken: {:?}, Nonce: {nonce}, {}H/S", duration, hash_rate);
        }
        hash
    }
}

#[derive(Debug)]
pub struct Account {
    addr: String,
    bal: u8,
}

#[derive(Debug)]
pub struct Blockchain<'a> {
    pub balances: Vec<Account>,
    pub chain: Vec<Block>,
    pub vm: &'a RandomXVM,
}

impl<'a> Blockchain<'a> {
    fn new(balances: Vec<Account>, vm: &'a RandomXVM) -> Blockchain<'a> {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            balances,
            vm,
        };
        blockchain.add_block("Master".to_string(), &mut vec![]);
        blockchain
    }

    fn add_block(&mut self, miner: String, transactions: &mut Vec<Trasaction>) {
        let index = self.chain.len() as u32;
        let previous_hash = if index == 0 {
            String::from("0")
        } else {
            self.chain[index as usize - 1].hash.clone()
        };

        let data = if index == 0 {
            "Genesis Block".to_string()
        } else {
            let mut data = String::new();
            let coinbase = Trasaction {
                timestamp: 0,
                from: "Master".to_string(),
                to: miner.to_string(),
                value: 10,
                data: "".into(),
            };
            transactions.push(coinbase);
            for tran in transactions {
                let acc_bal = self
                    .balances
                    .iter()
                    .find_map(|acc| {
                        if acc.addr == tran.from {
                            return Some(acc.bal);
                        }
                        None
                    })
                    .unwrap_or(0);
                if acc_bal < (tran.value as u8) {
                    println!("Not Enough Balance in {} account\n", tran.from);
                    continue;
                }
                let s = tran.to_str();
                println!("{}", &s);
                data.push_str(&s);
                self.update_bal(tran.from.clone(), Some(tran.value as u8), true);
                self.update_bal(tran.to.clone(), Some(tran.value as u8), false);
            }
            data
        };
        let btc_hash = String::from(*HASHES.get(index as usize).unwrap());
        let block: Block = Block::new(index, data, previous_hash, btc_hash, 4, self.vm);
        self.update_bal(miner, None, false);

        println!("Hash: {:?}, Data: {:?}\n", block.hash, block.data);
        self.chain.push(block);
    }

    fn get_bal(&mut self, addr: &str) -> Option<&mut Account> {
        self.balances.iter_mut().find(|acc| &acc.addr == addr)
    }

    fn update_bal(&mut self, addr: String, bal: Option<u8>, reduce: bool) {
        if let Some(a) = self.get_bal(&addr) {
            if let Some(bal) = bal {
                if reduce {
                    a.bal -= bal;
                } else {
                    a.bal += bal;
                }
            } else {
                a.bal += 10;
            }
        } else {
            self.balances.push(Account { addr, bal: 10 });
        }
    }
}

fn main() {
    let balances = BALANCES
        .iter()
        .map(|(name, bal)| {
            let addr = name.to_string();
            let bal = bal.clone();
            Account { addr, bal }
        })
        .collect::<Vec<_>>();
    let vm = &*VM;
    let mut blockchain = Blockchain::new(balances, vm);
    println!("Balances: {:?}", blockchain.balances);

    let mut transctions_1 = vec![Trasaction {
        timestamp: 1,
        from: "Alice".into(),
        to: "Bob".into(),
        data: "Block 1 Data".into(),
        value: 10,
    }];

    blockchain.add_block("Bob".into(), &mut transctions_1);
    println!("Balances: {:?}", blockchain.balances);

    let mut transctions_2 = vec![Trasaction {
        timestamp: 2,
        from: "Bob".into(),
        to: "Cathrine".into(),
        data: "Block 2 Data".into(),
        value: 5,
    }];
    blockchain.add_block("Bob".to_string(), &mut transctions_2);
    println!("Balances: {:?}", blockchain.balances);

    let mut transctions_2 = vec![Trasaction {
        timestamp: 3,
        from: "Cathrine".into(),
        to: "Dave".into(),
        data: "Block 3 Data".into(),
        value: 5,
    }];
    blockchain.add_block("Bob".to_string(), &mut transctions_2);
    println!("Balances: {:?}", blockchain.balances);

    let mut transctions_2 = vec![Trasaction {
        timestamp: 4,
        from: "Alice".into(),
        to: "Dave".into(),
        data: "Block 3 Data".into(),
        value: 5,
    }];
    blockchain.add_block("Bob".to_string(), &mut transctions_2);
    println!("Balances: {:?}", blockchain.balances);
}
