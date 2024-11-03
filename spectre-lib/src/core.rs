use crate::types::BlockTrace;
use crate::utils::*;
use alloy_eips::eip2930::AccessList;
use alloy_primitives::{Address, BlockHash, Bytes, B256, U256, U64};
use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer_local::LocalSigner;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;

#[derive(Debug, thiserror::Error)]
pub enum SpectreError {
    #[error("Error while tracing: {0}")]
    TracingError(String),
    #[error("Error while serializing/deserializing JSON: {0}")]
    SerdeError(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct Spectre {
    pub(crate) system: SystemConfig,
    pub(crate) block: Block,
    pub(crate) accounts: Accounts,
    pub(crate) transactions: Vec<Transaction>,
}

#[derive(Debug)]
pub struct SystemConfig {
    pub(crate) rng: StdRng,
    pub(crate) chain_id: u64,
    pub(crate) l1_queue_index: u64,
    pub(crate) history_hashes: Vec<BlockHash>,
    pub(crate) default_balance: Option<Ether>,
    pub(crate) default_gas_price: Option<Ether>,
    pub(crate) default_gas_limit: Option<u64>,
    pub(crate) logger: LoggerConfig,
}

#[derive(Debug, Serialize)]
pub(crate) struct Block {
    pub(crate) coinbase: Address,
    pub(crate) timestamp: U64,
    pub(crate) number: U256,
    pub(crate) difficulty: U256,
    pub(crate) gas_limit: U256,
    pub(crate) base_fee: U256,
}

pub(crate) struct Accounts {
    pub(crate) accounts: BTreeMap<Address, Account>,
    pub(crate) nonces: HashMap<Address, u64>,
    pub(crate) wallets: HashMap<Address, LocalSigner<SigningKey>>,
    pub(crate) aliases: HashMap<String, Address>,
}

#[derive(Serialize)]
struct TraceConfig {
    chain_id: u64,
    history_hashes: Vec<BlockHash>,
    block_constants: Block,
    accounts: BTreeMap<Address, Account>,
    transactions: Vec<Transaction>,
    logger_config: LoggerConfig,
    l1_queue_index: u64,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct LoggerConfig {
    #[serde(default = "crate::utils::default_false")]
    enable_memory: bool,
    #[serde(default = "crate::utils::default_true")]
    disable_stack: bool,
    #[serde(default = "crate::utils::default_true")]
    disable_storage: bool,
    #[serde(default = "crate::utils::default_false")]
    enable_return_data: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct Account {
    pub(crate) address: Address,
    pub(crate) nonce: U256,
    pub(crate) balance: U256,
    pub(crate) code: Bytes,
    pub(crate) storage: BTreeMap<B256, B256>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Transaction {
    pub(crate) from: Address,
    pub(crate) to: Option<Address>,
    pub(crate) nonce: U64,
    pub(crate) value: U256,
    pub(crate) gas_limit: U256,
    pub(crate) gas_price: Option<U256>,
    pub(crate) gas_fee_cap: Option<U256>,
    pub(crate) gas_tip_cap: Option<U256>,
    pub(crate) call_data: Bytes,
    pub(crate) access_list: AccessList,
    pub(crate) tx_type: &'static str,
    pub(crate) v: u64,
    pub(crate) r: U256,
    pub(crate) s: U256,
}

impl Spectre {
    pub fn trace(self) -> Result<BlockTrace, SpectreError> {
        let config = TraceConfig {
            chain_id: self.system.chain_id,
            history_hashes: self.system.history_hashes,
            block_constants: self.block,
            accounts: self.accounts.accounts,
            transactions: self.transactions,
            logger_config: self.system.logger,
            l1_queue_index: self.system.l1_queue_index,
        };
        let config_json = serde_json::to_string(&config)?;
        let trace_json = geth_utils::l2trace(&config_json).map_err(SpectreError::TracingError)?;
        Ok(serde_json::from_str(&trace_json)?)
    }
}

impl Accounts {
    pub(crate) fn fetch_add_nonce(&mut self, address: &Address) -> Option<u64> {
        self.nonces.get_mut(address).map(|nonce| {
            let old = *nonce;
            *nonce += 1;
            old
        })
    }

    pub(crate) fn resolve_wallet(
        &self,
        address: &AddressOrAlias,
    ) -> Option<&LocalSigner<SigningKey>> {
        match address {
            AddressOrAlias::Address(address) => self.wallets.get(address),
            AddressOrAlias::Alias(alias) => self
                .aliases
                .get(alias)
                .and_then(|address| self.wallets.get(address)),
        }
    }

    pub(crate) fn resolve_address(&self, address: &AddressOrAlias) -> Option<Address> {
        match address {
            AddressOrAlias::Address(address) => Some(*address),
            AddressOrAlias::Alias(alias) => self.aliases.get(alias).copied(),
        }
    }
}

impl Debug for Accounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accounts")
            .field("accounts", &self.accounts)
            .field("aliases", &self.aliases)
            .finish()
    }
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            enable_memory: false,
            disable_stack: true,
            disable_storage: true,
            enable_return_data: false,
        }
    }
}

#[cfg(feature = "cli")]
mod display {
    use super::*;
    use console::{style, Emoji};
    use std::fmt::Display;

    impl Display for Spectre {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(
                f,
                "{}{}",
                style("Loaded Spectre").bold().blue(),
                Emoji(" 👻", "")
            )?;
            write!(f, "{}", self.system)?;
            writeln!(f, "{}", self.block)?;
            writeln!(
                f,
                "{} {} accounts:",
                Emoji("💳", ""),
                self.accounts.accounts.len()
            )?;
            for (address, account) in &self.accounts.accounts {
                if !self.accounts.wallets.contains_key(address) {
                    writeln!(f, "{} {account}", Emoji("- 👤", "- [address]"))?;
                } else {
                    writeln!(f, "{} {account}", Emoji("- 🔐", "- [ wallet]"))?;
                }
            }
            writeln!(
                f,
                "\n{} {} transactions:",
                Emoji("💸", ""),
                self.transactions.len()
            )?;
            for tx in &self.transactions {
                writeln!(f, "- {}", tx)?;
            }
            Ok(())
        }
    }

    impl Display for SystemConfig {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            static NOT_AVAILABLE: Emoji<'_, '_> = Emoji("🚫", "");
            writeln!(f, "- {}Chain ID: {}", Emoji("🔗 ", ""), self.chain_id)?;
            writeln!(
                f,
                "- {}L1 Queue Index: {}",
                Emoji("📝 ", ""),
                self.l1_queue_index
            )?;
            if self.history_hashes.is_empty() {
                writeln!(f, "- {}History Hashes: {}", Emoji("📜 ", ""), NOT_AVAILABLE)?;
            } else {
                writeln!(
                    f,
                    "- {}History Hashes: {:?}",
                    Emoji("📜 ", ""),
                    self.history_hashes
                )?;
            }
            match self.default_balance {
                Some(balance) => {
                    writeln!(f, "- {}Default Balance: {:?}", Emoji("💵 ", ""), balance)?
                }
                None => writeln!(
                    f,
                    "- {}Default Balance: {}",
                    Emoji("💵 ", ""),
                    NOT_AVAILABLE
                )?,
            }
            match self.default_gas_price {
                Some(price) => writeln!(f, "- {}Default Gas Price: {:?}", Emoji("💸 ", ""), price)?,
                None => writeln!(
                    f,
                    "- {}Default Gas Price: {}",
                    Emoji("💸", ""),
                    NOT_AVAILABLE
                )?,
            };
            match self.default_gas_limit {
                Some(limit) => writeln!(f, "- {}Default Gas Limit: {:?}", Emoji("🛢️", ""), limit)?,
                None => writeln!(
                    f,
                    "- {}Default Gas Limit: {}",
                    Emoji("🛢️", ""),
                    NOT_AVAILABLE
                )?,
            }
            Ok(())
        }
    }

    impl Display for Block {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "- {}Coinbase: {}", Emoji("👛 ", ""), self.coinbase)?;
            writeln!(f, "- {}Timestamp: {}", Emoji("🕒 ", ""), self.timestamp)?;
            writeln!(f, "- {}Number: {}", Emoji("🔢 ", ""), self.number)?;
            writeln!(f, "- {}Difficulty: {}", Emoji("📈 ", ""), self.difficulty)?;
            writeln!(f, "- {}Gas Limit: {}", Emoji("🛢️", ""), self.gas_limit)?;
            writeln!(f, "- {}Base Fee: {}", Emoji("💸 ", ""), self.base_fee)?;
            Ok(())
        }
    }

    impl Display for Account {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}: {}{:?}",
                self.address,
                Emoji("💵", ""),
                Ether(self.balance),
            )?;
            write!(f, " | {}", Emoji("🔢", "Nonce: "))?;
            write!(f, "{: >5}", self.nonce)?;
            write!(f, " | {}", Emoji("🗄️", "Storage: "))?;
            if self.storage.is_empty() {
                write!(f, "{}", style("Empty").dim())?;
            } else {
                write!(f, "{: >5}", self.storage.len())?;
            }

            write!(f, " | {} ", style("</>").bold())?;
            if self.code.is_empty() {
                write!(f, "{: >11}", style("Empty").dim())?;
            } else {
                write!(f, "{: >5} bytes", self.code.len())?;
            }
            Ok(())
        }
    }

    impl Display for Transaction {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{: <7} | ", style(self.tx_type).bold().blue())?;
            write!(f, "{} -> ", self.from)?;
            if let Some(to) = self.to {
                write!(f, "{}", to)?;
            } else {
                write!(f, "{}", style("CREATE").bold())?;
            }
            write!(f, " | {:?}", Ether(self.value))?;
            Ok(())
        }
    }
}
