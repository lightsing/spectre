use crate::{Spectre, utils::*};
use alloy_consensus::{TxEip1559, TxEip2930, TxLegacy, TxType};
use alloy_genesis::{ChainConfig, Genesis, GenesisAccount};
use alloy_primitives::{Address, B256, Bytes, ChainId, TxKind, U256};
use alloy_rpc_types_eth::{AccessList, BlobTransactionSidecar, TransactionInput};
use alloy_serde::{OtherFields, WithOtherFields};
use alloy_signer_local::PrivateKeySigner;
use rand::{SeedableRng, rngs::StdRng};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    str::FromStr,
    time,
};

#[cfg(not(feature = "scroll"))]
use alloy_consensus::{TxEip4844Variant, TxEnvelope, TypedTransaction};
#[cfg(feature = "scroll")]
use scroll_alloy_consensus::{
    ScrollTxEnvelope as TxEnvelope, ScrollTypedTransaction as TypedTransaction,
};

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    // wallet errors
    #[error("Invalid secret of wallet#{idx}")]
    InvalidSecret { idx: usize },

    // alloc errors
    #[error("Invalid address of alloc#{idx}")]
    InvalidAddress { idx: usize },
    #[error("alloc#{idx}: wallet not found: {name}")]
    AllocWalletNotFound { idx: usize, name: String },
    #[error("alloc#{idx}: Neither balance or defaults is set")]
    BalanceNotSet { idx: usize },
    #[error("cannot compile code of alloc#{idx}({address:?}): {error:?}")]
    CompileError {
        idx: usize,
        address: Address,
        error: CompileError,
    },

    #[error("at least one transaction is required")]
    AtLeastOneTransaction,
    #[error("transaction#{idx}: unexpected tx type {tx_type}")]
    UnexpectedTxType { idx: usize, tx_type: u8 },
    #[error("transaction#{idx}: access list not set for eip2930 or eip1559 tx")]
    AccessListNotSet { idx: usize },
    #[error("transaction#{idx}: Account not found: {name}")]
    TxAccountNotFound { idx: usize, name: String },

    #[error("transaction#{idx}: Both gas price and default are not set")]
    GasPriceNotSet { idx: usize },
    #[error("transaction#{idx}: Both max fee per gas and default are not set")]
    MaxFeePerGasNotSet { idx: usize },
    #[error("transaction#{idx}: Both max priority fee per gas and default are not set")]
    MaxPriorityFeePerGasNotSet { idx: usize },
    #[error("transaction#{idx}: Both gas limit and default are not set")]
    GasLimitNotSet { idx: usize },
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpectreBuilder {
    #[serde(default)]
    pub system: SystemBuilder,
    #[serde(default)]
    pub defaults: DefaultsBuilder,
    #[serde(default)]
    pub genesis: GenesisBuilder,
    #[serde(default)]
    pub chain: ChainConfigBuilder,
    #[serde(default)]
    pub alloc: Vec<AllocBuilder>,
    #[serde(default)]
    pub wallet: Vec<WalletBuilder>,
    #[serde(default)]
    pub transactions: Vec<TransactionBuilder>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SystemBuilder {
    #[serde(default)]
    pub random_seed: Option<u64>,
    #[serde(default)]
    pub geth_path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DefaultsBuilder {
    #[serde(default)]
    pub account_balance: Option<Ether>,
    #[serde(default)]
    pub tx_gas_price: Option<Ether>,
    #[serde(default)]
    pub tx_max_fee_per_gas: Option<Ether>,
    #[serde(default)]
    pub tx_max_priority_fee_per_gas: Option<Ether>,
    #[serde(default)]
    pub tx_gas_limit: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GenesisBuilder {
    #[serde(default = "default_zero")]
    pub nonce: u64,
    #[serde(default = "default_now")]
    pub timestamp: u64,
    #[serde(default = "default_block_gas_limit")]
    pub gas_limit: u64,
    #[serde(default = "default_difficulty")]
    pub difficulty: U256,
    #[serde(default)]
    pub mix_hash: B256,
    #[serde(default)]
    pub coinbase: Address,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChainConfigBuilder {
    #[serde(default = "default_chain_id")]
    pub chain_id: u64,

    #[serde(default = "default_enabled")]
    pub homestead_block: BoolOr<u64>,
    #[serde(default)]
    pub dao_fork_block: BoolOr<u64>,
    #[serde(default = "default_true")]
    pub dao_fork_support: bool,
    #[serde(default = "default_enabled")]
    pub eip150_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub eip155_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub eip158_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub byzantium_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub constantinople_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub petersburg_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub istanbul_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub muir_glacier_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub berlin_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub london_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub arrow_glacier_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub gray_glacier_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub merge_netsplit_block: BoolOr<u64>,
    #[serde(default = "default_enabled")]
    pub shanghai_time: BoolOr<u64>,

    #[cfg(feature = "scroll")]
    #[serde(default = "default_enabled")]
    pub curie_block: BoolOr<u64>,
    #[cfg(feature = "scroll")]
    #[serde(default = "default_enabled")]
    pub darwin_time: BoolOr<u64>,
    #[cfg(feature = "scroll")]
    #[serde(default = "default_enabled")]
    pub darwinv2_time: BoolOr<u64>,
    #[cfg(feature = "scroll")]
    #[serde(default = "default_enabled")]
    pub euclid_time: BoolOr<u64>,
    #[cfg(feature = "scroll")]
    #[serde(default = "default_enabled")]
    pub euclidv2_time: BoolOr<u64>,

    #[cfg_attr(feature = "scroll", serde(default))]
    #[cfg_attr(not(feature = "scroll"), serde(default = "default_enabled"))]
    pub cancun_time: BoolOr<u64>,
    #[cfg_attr(feature = "scroll", serde(default))]
    #[cfg_attr(not(feature = "scroll"), serde(default = "default_enabled"))]
    pub prague_time: BoolOr<u64>,
    #[cfg_attr(feature = "scroll", serde(default))]
    #[cfg_attr(not(feature = "scroll"), serde(default = "default_enabled"))]
    pub osaka_time: BoolOr<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AllocBuilder {
    pub address: String,
    #[serde(default)]
    pub nonce: Option<u64>,
    #[serde(default)]
    pub balance: Option<Ether>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub storage: BTreeMap<U256, U256>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WalletBuilder {
    pub name: String,
    #[serde(default)]
    pub secret: Option<B256>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransactionBuilder {
    #[serde(rename = "type")]
    #[serde(default)]
    pub transaction_type: u8,
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub gas_price: Option<Ether>,
    #[serde(default)]
    pub max_fee_per_gas: Option<Ether>,
    #[serde(default)]
    pub max_priority_fee_per_gas: Option<Ether>,
    #[serde(default)]
    pub gas_limit: Option<u64>,
    #[serde(default)]
    pub value: Option<Ether>,
    #[serde(default)]
    pub input: Option<Bytes>,
    #[serde(default)]
    pub access_list: Option<AccessList>,
}

impl SpectreBuilder {
    pub fn build(self) -> Result<Spectre, BuilderError> {
        // for deterministic tests
        let mut rng = if let Some(random_seed) = self.system.random_seed {
            StdRng::seed_from_u64(random_seed)
        } else {
            StdRng::from_entropy()
        };

        let wallets_by_name = self
            .wallet
            .into_iter()
            .enumerate()
            .map(|(idx, wallet)| {
                if let Some(secret) = wallet.secret {
                    PrivateKeySigner::from_bytes(&secret)
                        .map_err(|_| BuilderError::InvalidSecret { idx })
                } else {
                    Ok(PrivateKeySigner::random_with(&mut rng))
                }
                .map(|signer| (wallet.name, signer))
            })
            .collect::<Result<HashMap<String, PrivateKeySigner>, _>>()?;
        let wallets = wallets_by_name
            .iter()
            .map(|(_, wallet)| (wallet.address(), wallet.clone()))
            .collect::<HashMap<Address, PrivateKeySigner>>();

        let alloc = self
            .alloc
            .into_iter()
            .enumerate()
            .map(|(idx, alloc)| alloc.build_with(idx, &wallets_by_name, &self.defaults))
            .collect::<Result<BTreeMap<Address, GenesisAccount>, _>>()?;

        let chain_config = self.chain.build();
        let genesis = self.genesis.build_with(chain_config, alloc);

        if self.transactions.is_empty() {
            return Err(BuilderError::AtLeastOneTransaction);
        }
        let transactions = self
            .transactions
            .into_iter()
            .enumerate()
            .map(|(idx, transaction)| {
                transaction.build_with(idx, &genesis, &wallets_by_name, &self.defaults)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Spectre {
            rng,
            geth_path: self.system.geth_path,
            genesis,
            wallets,
            transactions,
        })
    }
}

impl GenesisBuilder {
    fn build_with(
        self,
        chain_config: ChainConfig,
        alloc: BTreeMap<Address, GenesisAccount>,
    ) -> Genesis {
        let genesis = Genesis {
            config: chain_config,
            nonce: self.nonce,
            timestamp: self.timestamp,
            extra_data: Default::default(),
            gas_limit: self.gas_limit,
            difficulty: self.difficulty,
            mix_hash: self.mix_hash,
            coinbase: self.coinbase,
            alloc,
            ..Default::default()
        };
        genesis
    }
}

impl ChainConfigBuilder {
    fn build(self) -> ChainConfig {
        #[allow(unused_mut)]
        let mut extra_fields = OtherFields::default();
        #[cfg(feature = "scroll")]
        {
            if let Some(curie_block) = self.curie_block.into_option() {
                extra_fields.insert("curieBlock".to_string(), curie_block.into());
            }
            if let Some(darwin_time) = self.darwin_time.into_option() {
                extra_fields.insert("darwinTime".to_string(), darwin_time.into());
            }
            if let Some(darwinv2_time) = self.darwinv2_time.into_option() {
                extra_fields.insert("darwinv2Time".to_string(), darwinv2_time.into());
            }
            if let Some(euclid_time) = self.euclid_time.into_option() {
                extra_fields.insert("euclidTime".to_string(), euclid_time.into());
            }
            if let Some(euclidv2_time) = self.euclidv2_time.into_option() {
                extra_fields.insert("euclidv2Time".to_string(), euclidv2_time.into());
            }
        };

        ChainConfig {
            chain_id: self.chain_id,
            homestead_block: self.homestead_block.into_option(),
            dao_fork_block: self.dao_fork_block.into_option(),
            dao_fork_support: self.dao_fork_support,
            eip150_block: self.eip150_block.into_option(),
            eip155_block: self.eip155_block.into_option(),
            eip158_block: self.eip158_block.into_option(),
            byzantium_block: self.byzantium_block.into_option(),
            constantinople_block: self.constantinople_block.into_option(),
            petersburg_block: self.petersburg_block.into_option(),
            istanbul_block: self.istanbul_block.into_option(),
            muir_glacier_block: self.muir_glacier_block.into_option(),
            berlin_block: self.berlin_block.into_option(),
            london_block: self.london_block.into_option(),
            arrow_glacier_block: self.arrow_glacier_block.into_option(),
            gray_glacier_block: self.gray_glacier_block.into_option(),
            merge_netsplit_block: self.merge_netsplit_block.into_option(),
            shanghai_time: self.shanghai_time.into_option(),
            cancun_time: self.cancun_time.into_option(),
            prague_time: self.prague_time.into_option(),
            osaka_time: self.osaka_time.into_option(),
            terminal_total_difficulty: None,
            terminal_total_difficulty_passed: false,
            ethash: None,
            clique: None,
            parlia: None,
            extra_fields,
            deposit_contract_address: None,
            blob_schedule: Default::default(),
        }
    }
}

impl AllocBuilder {
    fn build_with(
        self,
        idx: usize,
        wallets: &HashMap<String, PrivateKeySigner>,
        defaults: &DefaultsBuilder,
    ) -> Result<(Address, GenesisAccount), BuilderError> {
        let address =
            resolve_address(&self.address, wallets).ok_or(BuilderError::AllocWalletNotFound {
                idx,
                name: self.address.clone(),
            })?;

        let code = match self.code {
            Some(s) => Some(
                Bytes::from_str(&s)
                    .or_else(|_| compile_mnemonic(&s))
                    .map_err(|e| BuilderError::CompileError {
                        idx,
                        address,
                        error: e,
                    })?,
            ),
            None => None,
        };

        let account = GenesisAccount {
            nonce: self.nonce,
            balance: self
                .balance
                .or_else(|| defaults.account_balance)
                .ok_or(BuilderError::BalanceNotSet { idx })?
                .0,
            code,
            storage: Some(
                self.storage
                    .into_iter()
                    .map(|(k, v)| (B256::from(k.to_be_bytes()), B256::from(v.to_be_bytes())))
                    .collect(),
            ),
            private_key: None,
        };

        Ok((address, account))
    }
}

impl TransactionBuilder {
    fn build_with(
        self,
        idx: usize,
        genesis: &Genesis,
        wallets: &HashMap<String, PrivateKeySigner>,
        defaults: &DefaultsBuilder,
    ) -> Result<(Address, TypedTransaction), BuilderError> {
        let tx_type = TxType::try_from(self.transaction_type).map_err(|_| {
            BuilderError::UnexpectedTxType {
                idx,
                tx_type: self.transaction_type,
            }
        })?;

        let chain_id = genesis.config.chain_id;

        let from = resolve_address(&self.from, wallets).ok_or(BuilderError::TxAccountNotFound {
            idx,
            name: self.from.clone(),
        })?;
        let to = match self.to {
            Some(ref to) => Some(resolve_address(to, wallets).ok_or(
                BuilderError::TxAccountNotFound {
                    idx,
                    name: to.clone(),
                },
            )?),
            None => None,
        };

        let tx = match tx_type {
            TxType::Legacy => {
                let tx = TxLegacy {
                    chain_id: Some(chain_id),
                    nonce: 0,
                    gas_price: gas_price(idx, self.gas_price, defaults)?.to(),
                    gas_limit: gas_limit(idx, self.gas_limit, defaults)?,
                    to: tx_kind(to),
                    value: self.value.unwrap_or_default().0,
                    input: self.input.unwrap_or_default(),
                };
                TypedTransaction::Legacy(tx)
            }
            TxType::Eip2930 => {
                let tx = TxEip2930 {
                    chain_id,
                    nonce: 0,
                    gas_price: gas_price(idx, self.gas_price, defaults)?.to(),
                    gas_limit: gas_limit(idx, self.gas_limit, defaults)?,
                    to: tx_kind(to),
                    value: self.value.unwrap_or_default().0,
                    input: self.input.unwrap_or_default(),
                    access_list: self
                        .access_list
                        .ok_or(BuilderError::AccessListNotSet { idx })?,
                };
                TypedTransaction::Eip2930(tx)
            }
            TxType::Eip1559 => {
                let tx = TxEip1559 {
                    chain_id,
                    nonce: 0,
                    gas_limit: gas_limit(idx, self.gas_limit, defaults)?,
                    max_fee_per_gas: max_fee_per_gas(idx, self.max_fee_per_gas, defaults)?.to(),
                    max_priority_fee_per_gas: max_priority_fee_per_gas(
                        idx,
                        self.max_priority_fee_per_gas,
                        defaults,
                    )?
                    .to(),
                    to: tx_kind(to),
                    value: self.value.unwrap_or_default().0,
                    access_list: self
                        .access_list
                        .ok_or(BuilderError::AccessListNotSet { idx })?,
                    input: self.input.unwrap_or_default(),
                };
                TypedTransaction::Eip1559(tx)
            }
            _ => unimplemented!(),
        };

        Ok((from, tx))
    }
}

#[cfg(feature = "scroll")]
impl Default for ChainConfigBuilder {
    fn default() -> Self {
        ChainConfigBuilder {
            chain_id: default_chain_id(),
            homestead_block: BoolOr::Value(0),
            dao_fork_block: BoolOr::Bool(false),
            dao_fork_support: true,
            eip150_block: BoolOr::Value(0),
            eip155_block: BoolOr::Value(0),
            eip158_block: BoolOr::Value(0),
            byzantium_block: BoolOr::Value(0),
            constantinople_block: BoolOr::Value(0),
            petersburg_block: BoolOr::Value(0),
            istanbul_block: BoolOr::Value(0),
            muir_glacier_block: BoolOr::Value(0),
            berlin_block: BoolOr::Value(0),
            london_block: BoolOr::Value(0),
            arrow_glacier_block: BoolOr::Value(0),
            gray_glacier_block: BoolOr::Value(0),
            merge_netsplit_block: BoolOr::Value(0),
            shanghai_time: BoolOr::Value(0),
            curie_block: BoolOr::Value(0),
            darwin_time: BoolOr::Value(0),
            darwinv2_time: BoolOr::Value(0),
            euclid_time: BoolOr::Value(0),
            euclidv2_time: BoolOr::Value(0),
            cancun_time: BoolOr::Bool(false),
            prague_time: BoolOr::Bool(false),
            osaka_time: BoolOr::Bool(false),
        }
    }
}

#[cfg(not(feature = "scroll"))]
impl Default for ChainConfigBuilder {
    fn default() -> Self {
        ChainConfigBuilder {
            chain_id: default_chain_id(),
            homestead_block: BoolOr::Value(0),
            dao_fork_block: BoolOr::Value(0),
            dao_fork_support: true,
            eip150_block: BoolOr::Value(0),
            eip155_block: BoolOr::Value(0),
            eip158_block: BoolOr::Value(0),
            byzantium_block: BoolOr::Value(0),
            constantinople_block: BoolOr::Value(0),
            petersburg_block: BoolOr::Value(0),
            istanbul_block: BoolOr::Value(0),
            muir_glacier_block: BoolOr::Value(0),
            berlin_block: BoolOr::Value(0),
            london_block: BoolOr::Value(0),
            arrow_glacier_block: BoolOr::Value(0),
            gray_glacier_block: BoolOr::Value(0),
            merge_netsplit_block: BoolOr::Value(0),
            shanghai_time: BoolOr::Value(0),
            cancun_time: BoolOr::Value(0),
            prague_time: BoolOr::Value(0),
            osaka_time: BoolOr::Value(0),
        }
    }
}

const fn default_gas() -> u64 {
    10_000_000
}

const fn default_base_fee() -> Ether {
    Ether(U256::ZERO)
}

fn default_now() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn default_block_gas_limit() -> u64 {
    30_000_000
}

fn default_difficulty() -> U256 {
    U256::from_limbs([1, 0, 0, 0])
}

#[cfg(not(feature = "scroll"))]
fn default_chain_id() -> u64 {
    1337
}
#[cfg(feature = "scroll")]
fn default_chain_id() -> u64 {
    22222
}

fn resolve_address(address: &str, wallets: &HashMap<String, PrivateKeySigner>) -> Option<Address> {
    if address.starts_with("0x") {
        Address::from_str(address).ok()
    } else {
        wallets.get(address).map(|wallet| wallet.address())
    }
}

#[inline]
fn gas_price(
    idx: usize,
    gas_price: Option<Ether>,
    defaults: &DefaultsBuilder,
) -> Result<U256, BuilderError> {
    gas_price
        .or(defaults.tx_gas_price)
        .ok_or_else(|| BuilderError::GasPriceNotSet { idx })
        .map(|price| price.0)
}

#[inline]
fn max_fee_per_gas(
    idx: usize,
    max_fee_per_gas: Option<Ether>,
    defaults: &DefaultsBuilder,
) -> Result<U256, BuilderError> {
    max_fee_per_gas
        .or(defaults.tx_max_fee_per_gas)
        .ok_or_else(|| BuilderError::MaxFeePerGasNotSet { idx })
        .map(|price| price.0)
}

#[inline]
fn max_priority_fee_per_gas(
    idx: usize,
    max_priority_fee_per_gas: Option<Ether>,
    defaults: &DefaultsBuilder,
) -> Result<U256, BuilderError> {
    max_priority_fee_per_gas
        .or(defaults.tx_max_priority_fee_per_gas)
        .ok_or_else(|| BuilderError::MaxPriorityFeePerGasNotSet { idx })
        .map(|price| price.0)
}

#[inline]
fn gas_limit(
    idx: usize,
    gas_limit: Option<u64>,
    defaults: &DefaultsBuilder,
) -> Result<u64, BuilderError> {
    gas_limit
        .or(defaults.tx_gas_limit)
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

    #[tokio::test]
    async fn test_de_and_trace() {
        let config: SpectreBuilder =
            toml::from_str(include_str!("../../../examples/full.toml")).unwrap();
        config.build().unwrap().trace().await.unwrap();
    }
}
