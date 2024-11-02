use alloy_eips::eip2930::AccessListItem;
use alloy_primitives::{Address, Bytes, B256, U256, U64};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockTrace {
    #[serde(rename = "chainID", default)]
    pub chain_id: u64,
    pub coinbase: AccountTrace,
    pub header: BlockHeader,
    pub transactions: Vec<TransactionTrace>,
    pub codes: Vec<BytecodeTrace>,
    #[serde(rename = "storageTrace")]
    pub storage_trace: StorageTrace,
    #[serde(rename = "startL1QueueIndex", default)]
    pub start_l1_queue_index: u64,
    pub withdraw_trie_root: B256,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountTrace {
    pub address: Address,
    pub nonce: u64,
    pub balance: U256,
    #[serde(rename = "keccakCodeHash")]
    pub keccak_code_hash: B256,
    #[serde(rename = "poseidonCodeHash")]
    pub poseidon_code_hash: B256,
    #[serde(rename = "codeSize")]
    pub code_size: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BlockHeader {
    pub hash: B256,
    #[serde(rename = "miner")]
    pub author: Address,
    #[serde(rename = "stateRoot")]
    pub state_root: B256,
    pub number: U64,
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    #[serde(rename = "gasLimit")]
    pub gas_limit: U256,
    pub timestamp: U256,
    #[serde(default)]
    pub difficulty: U256,
    #[serde(default, rename = "mixHash")]
    pub mix_hash: Option<B256>,
    pub nonce: U64,
    #[serde(rename = "baseFeePerGas")]
    pub base_fee_per_gas: Option<U256>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BytecodeTrace {
    pub hash: B256,
    pub code: Bytes,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionTrace {
    #[serde(default, rename = "txHash")]
    pub tx_hash: B256,
    #[serde(rename = "type")]
    pub type_: u8,
    pub nonce: u64,
    pub gas: u64,
    #[serde(rename = "gasPrice")]
    pub gas_price: U256,
    #[serde(rename = "gasTipCap")]
    pub gas_tip_cap: Option<U256>,
    #[serde(rename = "gasFeeCap")]
    pub gas_fee_cap: Option<U256>,
    pub from: Address,
    pub to: Option<Address>,
    #[serde(rename = "chainId")]
    pub chain_id: U256,
    pub value: U256,
    pub data: Bytes,
    #[serde(rename = "isCreate")]
    pub is_create: bool,
    #[serde(rename = "accessList")]
    pub access_list: Option<Vec<AccessListItem>>,
    pub v: U64,
    pub r: U256,
    pub s: U256,
}

pub type AccountTrieProofs = BTreeMap<Address, Vec<Bytes>>;
pub type StorageTrieProofs = BTreeMap<Address, BTreeMap<B256, Vec<Bytes>>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageTrace {
    #[serde(rename = "rootBefore")]
    pub root_before: B256,
    #[serde(rename = "rootAfter")]
    pub root_after: B256,
    pub proofs: Option<AccountTrieProofs>,
    #[serde(rename = "storageProofs", default)]
    pub storage_proofs: StorageTrieProofs,
    #[serde(rename = "deletionProofs", default)]
    pub deletion_proofs: Vec<Bytes>,
    #[serde(rename = "flattenProofs", default)]
    pub flatten_proofs: BTreeMap<B256, Bytes>,
}
