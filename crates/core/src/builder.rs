use crate::{core::*, utils::*};
use alloy_consensus::{TxEip1559, TxEip2930, TxLegacy};
use alloy_eips::eip2930::{AccessList, AccessListItem};
use alloy_network::TxSignerSync;
use alloy_primitives::hex::FromHex;
use alloy_primitives::{
    Address, BlockHash, BlockNumber, BlockTimestamp, Bytes, ChainId, TxKind, B256, U256, U64,
};
use alloy_signer_local::PrivateKeySigner;
use rand::{rngs::StdRng, SeedableRng};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::time;

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Invalid secret of account#{idx}")]
    InvalidSecret { idx: usize },
    #[error("Cannot compile code of account#{idx}({address:?}): {error:?}")]
    CompileError {
        idx: usize,
        address: Address,
        error: CompileError,
    },
    #[error("Transaction#{idx}: Account not found: {address:?}")]
    AccountNotFound { idx: usize, address: AddressOrAlias },
    #[error("Transaction#{idx}: Both gas price and default are not set")]
    GasPriceNotSet { idx: usize },
    #[error("Transaction#{idx}: Both gas limit and default are not set")]
    GasLimitNotSet { idx: usize },
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpectreBuilder {
    #[serde(default)]
    pub system: SystemBuilder,
    #[serde(default)]
    pub block: BlockBuilder,
    #[serde(default)]
    pub accounts: Vec<AccountBuilder>,
    #[serde(default)]
    pub transactions: Vec<TransactionBuilder>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SystemBuilder {
    #[serde(default)]
    pub random_seed: Option<u64>,
    #[serde(default = "default_chain_id")]
    pub chain_id: ChainId,
    #[serde(default)]
    pub l1_queue_index: u64,
    #[serde(default)]
    pub history_hashes: Vec<BlockHash>,
    #[serde(default)]
    pub default_balance: Option<Ether>,
    #[serde(default)]
    pub default_gas_price: Option<Ether>,
    #[serde(default)]
    pub default_gas_limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BlockBuilder {
    #[serde(default)]
    pub coinbase: Address,
    #[serde(default = "default_block_number")]
    pub number: BlockNumber,
    #[serde(default = "default_timestamp")]
    pub timestamp: BlockTimestamp,
    #[serde(default = "default_gas")]
    pub gas_limit: u64,
    #[serde(default = "default_base_fee")]
    pub base_fee: Ether,
    #[serde(default)]
    pub difficulty: U256,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AccountBuilder {
    Wallet(AddressWalletBuilder),
    Address(AddressAccountBuilder),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddressAccountBuilder {
    pub address: Address,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub nonce: u64,
    #[serde(default)]
    pub balance: Option<Ether>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub storage: BTreeMap<U256, U256>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddressWalletBuilder {
    #[serde(rename = "wallet")]
    pub _marker: bool,
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub nonce: u64,
    #[serde(default)]
    pub balance: Option<Ether>,
    #[serde(default)]
    pub secret: Option<B256>,
    #[serde(default)]
    pub storage: BTreeMap<U256, U256>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TransactionBuilder {
    Eip155(LegacyTransactionBuilder),
    Eip1559(Eip1559TransactionBuilder),
    Eip2930(Eip2930TransactionBuilder),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LegacyTransactionBuilder {
    pub from: AddressOrAlias,
    #[serde(default)]
    pub to: Option<AddressOrAlias>,
    #[serde(default)]
    pub gas_price: Option<Ether>,
    #[serde(default)]
    pub gas_limit: Option<u64>,
    #[serde(default)]
    pub value: Ether,
    #[serde(default)]
    pub input: Bytes,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Eip1559TransactionBuilder {
    pub from: AddressOrAlias,
    #[serde(default)]
    pub to: Option<AddressOrAlias>,
    /// This is also known as `GasFeeCap`
    pub max_fee_per_gas: Ether,
    /// This is also known as `GasTipCap`
    pub max_priority_fee_per_gas: Ether,
    #[serde(default)]
    pub gas_limit: Option<u64>,
    #[serde(default)]
    pub value: Ether,
    #[serde(default)]
    pub access_list: HashMap<Address, Vec<B256>>,
    #[serde(default)]
    pub input: Bytes,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Eip2930TransactionBuilder {
    pub from: AddressOrAlias,
    #[serde(default)]
    pub to: Option<AddressOrAlias>,
    #[serde(default)]
    pub gas_price: Option<Ether>,
    #[serde(default)]
    pub gas_limit: Option<u64>,
    #[serde(default)]
    pub value: Ether,
    #[serde(default)]
    pub access_list: HashMap<Address, Vec<B256>>,
    #[serde(default)]
    pub input: Bytes,
}

impl SpectreBuilder {
    pub fn build(self) -> Result<Spectre, BuilderError> {
        let mut system = self.system.build();
        let block = self.block.build();
        let mut accounts = Accounts::build_from(&mut system, self.accounts)?;
        let transactions = self
            .transactions
            .into_iter()
            .enumerate()
            .map(|(idx, builder)| builder.build(idx, &system, &mut accounts))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Spectre {
            system,
            block,
            accounts,
            transactions,
        })
    }
}

impl SystemBuilder {
    fn build(self) -> SystemConfig {
        SystemConfig {
            rng: self
                .random_seed
                .map(StdRng::seed_from_u64)
                .unwrap_or_else(StdRng::from_entropy),
            chain_id: self.chain_id,
            l1_queue_index: self.l1_queue_index,
            history_hashes: self.history_hashes,
            default_balance: self.default_balance,
            default_gas_price: self.default_gas_price,
            default_gas_limit: self.default_gas_limit,
            logger: LoggerConfig::default(),
        }
    }
}

impl BlockBuilder {
    fn build(self) -> Block {
        Block {
            coinbase: self.coinbase,
            timestamp: U64::from(self.timestamp),
            number: U256::from(self.number),
            difficulty: self.difficulty,
            gas_limit: U256::from(self.gas_limit),
            base_fee: self.base_fee.0,
        }
    }
}

impl Accounts {
    fn build_from(
        system: &mut SystemConfig,
        builders: Vec<AccountBuilder>,
    ) -> Result<Self, BuilderError> {
        let mut accounts = BTreeMap::new();
        let mut nonces = HashMap::new();
        let mut wallets = HashMap::new();
        let mut aliases = HashMap::new();
        for (idx, builder) in builders.into_iter().enumerate() {
            match builder {
                AccountBuilder::Wallet(wallet) => {
                    let signer = match wallet.secret {
                        Some(secret) => PrivateKeySigner::from_bytes(&secret)
                            .map_err(|_| BuilderError::InvalidSecret { idx })?,
                        None => PrivateKeySigner::random_with(&mut system.rng),
                    };
                    let address = signer.address();
                    if let Some(alias) = wallet.alias {
                        aliases.insert(alias, address);
                    }
                    wallets.insert(address, signer);
                    let account = Account {
                        address,
                        nonce: U256::from(wallet.nonce),
                        balance: wallet
                            .balance
                            .or(system.default_balance)
                            .unwrap_or_default()
                            .0,
                        code: Bytes::default(),
                        storage: wallet
                            .storage
                            .into_iter()
                            .map(|(k, v)| (k.to_be_bytes().into(), v.to_be_bytes().into()))
                            .collect(),
                    };
                    nonces.insert(address, wallet.nonce);
                    accounts.insert(address, account);
                }
                AccountBuilder::Address(account) => {
                    let address = account.address;
                    if let Some(alias) = account.alias {
                        aliases.insert(alias, address);
                    }
                    let code = account
                        .code
                        .map(|c| Bytes::from_hex(&c).or_else(|_| compile_mnemonic(&c)))
                        .transpose()
                        .map_err(|error| BuilderError::CompileError {
                            idx,
                            address,
                            error,
                        })?
                        .unwrap_or_default();
                    let account = Account {
                        address,
                        nonce: U256::from(account.nonce),
                        balance: account
                            .balance
                            .or(system.default_balance)
                            .unwrap_or_default()
                            .0,
                        code,
                        storage: account
                            .storage
                            .into_iter()
                            .map(|(k, v)| (k.to_be_bytes().into(), v.to_be_bytes().into()))
                            .collect(),
                    };
                    accounts.insert(address, account);
                }
            }
        }

        Ok(Accounts {
            accounts,
            nonces,
            wallets,
            aliases,
        })
    }
}

impl TransactionBuilder {
    fn build(
        self,
        idx: usize,
        system: &SystemConfig,
        accounts: &mut Accounts,
    ) -> Result<Transaction, BuilderError> {
        match self {
            TransactionBuilder::Eip155(builder) => builder.build(idx, system, accounts),
            TransactionBuilder::Eip1559(builder) => builder.build(idx, system, accounts),
            TransactionBuilder::Eip2930(builder) => builder.build(idx, system, accounts),
        }
    }
}

impl LegacyTransactionBuilder {
    fn build(
        self,
        idx: usize,
        system: &SystemConfig,
        accounts: &mut Accounts,
    ) -> Result<Transaction, BuilderError> {
        let from_address =
            accounts
                .resolve_address(&self.from)
                .ok_or_else(|| BuilderError::AccountNotFound {
                    idx,
                    address: self.from.clone(),
                })?;
        let from_nonce = accounts.fetch_add_nonce(&from_address).unwrap();
        let to_address = self
            .to
            .map(|to| {
                accounts
                    .resolve_address(&to)
                    .ok_or_else(|| BuilderError::AccountNotFound { idx, address: to })
            })
            .transpose()?;
        let signer =
            accounts
                .resolve_wallet(&self.from)
                .ok_or_else(|| BuilderError::AccountNotFound {
                    idx: 0,
                    address: self.from,
                })?;
        let gas_price = gas_price(idx, self.gas_price, system)?;
        let mut tx = TxLegacy {
            chain_id: Some(system.chain_id),
            nonce: from_nonce,
            gas_price: gas_price.to(),
            gas_limit: gas_limit(idx, self.gas_limit, system)?,
            to: tx_kind(to_address),
            value: self.value.0,
            input: self.input.clone(),
        };
        let sig = signer.sign_transaction_sync(&mut tx).unwrap();
        Ok(Transaction {
            from: from_address,
            to: to_address,
            nonce: U64::from(tx.nonce),
            value: tx.value,
            gas_limit: U256::from(tx.gas_limit),
            gas_price: Some(gas_price),
            gas_fee_cap: None,
            gas_tip_cap: None,
            call_data: self.input.clone(),
            access_list: Default::default(),
            tx_type: "Eip155",
            v: sig.v().to_u64(),
            r: sig.r(),
            s: sig.s(),
        })
    }
}

impl Eip1559TransactionBuilder {
    fn build(
        self,
        idx: usize,
        system: &SystemConfig,
        accounts: &mut Accounts,
    ) -> Result<Transaction, BuilderError> {
        let from_address =
            accounts
                .resolve_address(&self.from)
                .ok_or_else(|| BuilderError::AccountNotFound {
                    idx,
                    address: self.from.clone(),
                })?;
        let from_nonce = accounts.fetch_add_nonce(&from_address).unwrap();
        let to_address = self
            .to
            .map(|to| {
                accounts
                    .resolve_address(&to)
                    .ok_or_else(|| BuilderError::AccountNotFound { idx, address: to })
            })
            .transpose()?;
        let signer =
            accounts
                .resolve_wallet(&self.from)
                .ok_or_else(|| BuilderError::AccountNotFound {
                    idx: 0,
                    address: self.from,
                })?;
        let access_list = AccessList::from(
            self.access_list
                .into_iter()
                .map(|(address, storage_keys)| AccessListItem {
                    address,
                    storage_keys,
                })
                .collect::<Vec<_>>(),
        );
        let mut tx = TxEip1559 {
            chain_id: system.chain_id,
            nonce: from_nonce,
            gas_limit: gas_limit(idx, self.gas_limit, system)?,
            max_fee_per_gas: self.max_fee_per_gas.0.to(),
            max_priority_fee_per_gas: self.max_priority_fee_per_gas.0.to(),
            to: tx_kind(to_address),
            value: self.value.0,
            access_list: access_list.clone(),
            input: self.input.clone(),
        };
        let sig = signer.sign_transaction_sync(&mut tx).unwrap();
        Ok(Transaction {
            from: from_address,
            to: to_address,
            nonce: U64::from(tx.nonce),
            value: tx.value,
            gas_limit: U256::from(tx.gas_limit),
            gas_price: None,
            gas_fee_cap: Some(self.max_fee_per_gas.0),
            gas_tip_cap: Some(self.max_priority_fee_per_gas.0),
            call_data: self.input,
            access_list,
            tx_type: "Eip1559",
            v: sig.v().to_u64(),
            r: sig.r(),
            s: sig.s(),
        })
    }
}

impl Eip2930TransactionBuilder {
    fn build(
        self,
        idx: usize,
        system: &SystemConfig,
        accounts: &mut Accounts,
    ) -> Result<Transaction, BuilderError> {
        let from_address =
            accounts
                .resolve_address(&self.from)
                .ok_or_else(|| BuilderError::AccountNotFound {
                    idx,
                    address: self.from.clone(),
                })?;
        let from_nonce = accounts.fetch_add_nonce(&from_address).unwrap();
        let to_address = self
            .to
            .map(|to| {
                accounts
                    .resolve_address(&to)
                    .ok_or_else(|| BuilderError::AccountNotFound { idx, address: to })
            })
            .transpose()?;
        let signer =
            accounts
                .resolve_wallet(&self.from)
                .ok_or_else(|| BuilderError::AccountNotFound {
                    idx: 0,
                    address: self.from,
                })?;
        let gas_price = gas_price(idx, self.gas_price, system)?;
        let access_list = AccessList::from(
            self.access_list
                .into_iter()
                .map(|(address, storage_keys)| AccessListItem {
                    address,
                    storage_keys,
                })
                .collect::<Vec<_>>(),
        );
        let mut tx = TxEip2930 {
            chain_id: system.chain_id,
            nonce: from_nonce,
            gas_price: gas_price.to(),
            gas_limit: gas_limit(idx, self.gas_limit, system)?,
            to: tx_kind(to_address),
            value: self.value.0,
            access_list: access_list.clone(),
            input: self.input.clone(),
        };
        let sig = signer.sign_transaction_sync(&mut tx).unwrap();
        Ok(Transaction {
            from: from_address,
            to: to_address,
            nonce: U64::from(tx.nonce),
            value: tx.value,
            gas_limit: U256::from(tx.gas_limit),
            gas_price: Some(gas_price),
            gas_fee_cap: None,
            gas_tip_cap: None,
            call_data: self.input,
            access_list,
            tx_type: "Eip2930",
            v: sig.v().to_u64(),
            r: sig.r(),
            s: sig.s(),
        })
    }
}
impl Default for SystemBuilder {
    fn default() -> Self {
        Self {
            random_seed: None,
            chain_id: default_chain_id(),
            l1_queue_index: 0,
            history_hashes: vec![],
            default_balance: None,
            default_gas_price: None,
            default_gas_limit: None,
        }
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self {
            coinbase: Address::ZERO,
            number: default_block_number(),
            timestamp: default_timestamp(),
            gas_limit: default_gas(),
            base_fee: default_base_fee(),
            difficulty: U256::ZERO,
        }
    }
}

const fn default_gas() -> u64 {
    10_000_000
}

const fn default_chain_id() -> ChainId {
    1
}

const fn default_block_number() -> BlockNumber {
    0xcafe
}

fn default_timestamp() -> BlockTimestamp {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

const fn default_base_fee() -> Ether {
    Ether(U256::ZERO)
}

#[inline]
fn gas_price(
    idx: usize,
    gas_price: Option<Ether>,
    system: &SystemConfig,
) -> Result<U256, BuilderError> {
    gas_price
        .or(system.default_gas_price)
        .ok_or_else(|| BuilderError::GasPriceNotSet { idx })
        .map(|price| price.0)
}

#[inline]
fn gas_limit(
    idx: usize,
    gas_limit: Option<u64>,
    system: &SystemConfig,
) -> Result<u64, BuilderError> {
    gas_limit
        .or(system.default_gas_limit)
        .ok_or_else(|| BuilderError::GasLimitNotSet { idx })
}

#[inline]
fn tx_kind(to_address: Option<Address>) -> TxKind {
    match to_address {
        Some(address) => TxKind::Call(address),
        None => TxKind::Create,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_de_and_trace() {
        let config: SpectreBuilder = toml::from_str(include_str!("../example.toml")).unwrap();
        config.build().unwrap().trace().unwrap();
    }
}
