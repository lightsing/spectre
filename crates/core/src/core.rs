use alloy_consensus::SignableTransaction;
use alloy_genesis::Genesis;
use alloy_network::{ReceiptResponse, TxSignerSync};
use alloy_primitives::Address;
use alloy_provider::{Provider, network::TransactionBuilder};
use alloy_serde::WithOtherFields;
use alloy_signer_local::PrivateKeySigner;
use rand::prelude::StdRng;
use sbv_primitives::types::ExecutionWitness;
use std::{collections::HashMap, fmt::Debug, path::PathBuf};

#[cfg(not(feature = "scroll"))]
use alloy_consensus::{TxEip4844Variant, TxEnvelope, TypedTransaction};
use sbv_utils::rpc::ProviderExt;
#[cfg(feature = "scroll")]
use scroll_alloy_consensus::{
    ScrollTxEnvelope as TxEnvelope, ScrollTypedTransaction as TypedTransaction,
};

#[derive(Debug, thiserror::Error)]
pub enum SpectreError {
    #[error("Error while build testnet: {0}")]
    TestnetBuilder(#[from] testnet::TestNetBuilderError),
    #[error("Error while serializing/deserializing JSON: {0}")]
    Serde(#[from] serde_json::Error),
}

// #[derive(Debug)]
pub struct Spectre {
    pub(crate) rng: StdRng,
    pub(crate) geth_path: PathBuf,
    pub(crate) genesis: Genesis,
    pub(crate) wallets: HashMap<Address, PrivateKeySigner>,
    pub(crate) transactions: Vec<(Address, TypedTransaction)>,
}

impl Spectre {
    pub async fn trace(self) -> Result<Vec<ExecutionWitness>, SpectreError> {
        let provider = testnet::TestNetBuilder::default()
            .geth_path(self.geth_path)
            .genesis(self.genesis)
            .build()
            .await?;

        let witnesses = vec![];
        for (from, mut tx) in self.transactions.into_iter() {
            let nonce = provider.get_transaction_count(from).await.unwrap();
            let signer = self.wallets.get(&from).expect("missing wallet");
            let tx_envelope = match tx {
                TypedTransaction::Legacy(mut tx) => {
                    tx.nonce = nonce;
                    let sig = signer.sign_transaction_sync(&mut tx).unwrap();
                    TxEnvelope::Legacy(tx.into_signed(sig))
                }
                TypedTransaction::Eip2930(mut tx) => {
                    tx.nonce = nonce;
                    let sig = signer.sign_transaction_sync(&mut tx).unwrap();
                    TxEnvelope::Eip2930(tx.into_signed(sig))
                }
                TypedTransaction::Eip1559(mut tx) => {
                    tx.nonce = nonce;
                    let sig = signer.sign_transaction_sync(&mut tx).unwrap();
                    TxEnvelope::Eip1559(tx.into_signed(sig))
                }
                #[cfg(not(feature = "scroll"))]
                TypedTransaction::Eip4844(mut tx) => {
                    match &mut tx {
                        TxEip4844Variant::TxEip4844(tx) => {
                            tx.nonce = nonce;
                        }
                        TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                            tx.tx.nonce = nonce;
                        }
                    }

                    let sig = signer.sign_transaction_sync(&mut tx).unwrap();
                    TxEnvelope::Eip4844(tx.into_signed(sig))
                }
                TypedTransaction::Eip7702(mut tx) => {
                    tx.nonce = nonce;
                    let sig = signer.sign_transaction_sync(&mut tx).unwrap();
                    TxEnvelope::Eip7702(tx.into_signed(sig))
                }
                _ => unimplemented!(),
            };

            trace!(tx_envelope = ?tx_envelope);
            let mut pending_tx = provider.send_tx_envelope(tx_envelope).await.unwrap();
            pending_tx.set_required_confirmations(0);
            let receipt = pending_tx.get_receipt().await.unwrap();
            trace!(receipt = ?receipt);

            let block_number = receipt.block_number().unwrap();
            let block = provider
                .dump_block_witness(block_number.into())
                .await
                .unwrap();
        }

        Ok(witnesses)
    }
}

// #[cfg(feature = "cli")]
// mod display {
//     use super::*;
//     use console::{Emoji, style};
//     use std::fmt::Display;
//
//     impl Display for Spectre {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             writeln!(
//                 f,
//                 "{}{}",
//                 style("Loaded Spectre").bold().blue(),
//                 Emoji(" 👻", "")
//             )?;
//             write!(f, "{}", self.system)?;
//             writeln!(f, "{}", self.block)?;
//             writeln!(
//                 f,
//                 "{} {} accounts:",
//                 Emoji("💳", ""),
//                 self.accounts.accounts.len()
//             )?;
//             for (address, account) in &self.accounts.accounts {
//                 if !self.accounts.wallets.contains_key(address) {
//                     writeln!(f, "{} {account}", Emoji("- 👤", "- [address]"))?;
//                 } else {
//                     writeln!(f, "{} {account}", Emoji("- 🔐", "- [ wallet]"))?;
//                 }
//             }
//             writeln!(
//                 f,
//                 "\n{} {} transactions:",
//                 Emoji("💸", ""),
//                 self.transactions.len()
//             )?;
//             for tx in &self.transactions {
//                 writeln!(f, "- {}", tx)?;
//             }
//             Ok(())
//         }
//     }
//
//     impl Display for SystemConfig {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             static NOT_AVAILABLE: Emoji<'_, '_> = Emoji("🚫", "");
//             writeln!(f, "- {}Chain ID: {}", Emoji("🔗 ", ""), self.chain_id)?;
//             writeln!(
//                 f,
//                 "- {}L1 Queue Index: {}",
//                 Emoji("📝 ", ""),
//                 self.l1_queue_index
//             )?;
//             if self.history_hashes.is_empty() {
//                 writeln!(f, "- {}History Hashes: {}", Emoji("📜 ", ""), NOT_AVAILABLE)?;
//             } else {
//                 writeln!(
//                     f,
//                     "- {}History Hashes: {:?}",
//                     Emoji("📜 ", ""),
//                     self.history_hashes
//                 )?;
//             }
//             match self.default_balance {
//                 Some(balance) => {
//                     writeln!(f, "- {}Default Balance: {:?}", Emoji("💵 ", ""), balance)?
//                 }
//                 None => writeln!(
//                     f,
//                     "- {}Default Balance: {}",
//                     Emoji("💵 ", ""),
//                     NOT_AVAILABLE
//                 )?,
//             }
//             match self.default_gas_price {
//                 Some(price) => writeln!(f, "- {}Default Gas Price: {:?}", Emoji("💸 ", ""), price)?,
//                 None => writeln!(
//                     f,
//                     "- {}Default Gas Price: {}",
//                     Emoji("💸", ""),
//                     NOT_AVAILABLE
//                 )?,
//             };
//             match self.default_gas_limit {
//                 Some(limit) => writeln!(f, "- {}Default Gas Limit: {:?}", Emoji("🛢️", ""), limit)?,
//                 None => writeln!(
//                     f,
//                     "- {}Default Gas Limit: {}",
//                     Emoji("🛢️", ""),
//                     NOT_AVAILABLE
//                 )?,
//             }
//             Ok(())
//         }
//     }
//
//     impl Display for Block {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             writeln!(f, "- {}Coinbase: {}", Emoji("👛 ", ""), self.coinbase)?;
//             writeln!(f, "- {}Timestamp: {}", Emoji("🕒 ", ""), self.timestamp)?;
//             writeln!(f, "- {}Number: {}", Emoji("🔢 ", ""), self.number)?;
//             writeln!(f, "- {}Difficulty: {}", Emoji("📈 ", ""), self.difficulty)?;
//             writeln!(f, "- {}Gas Limit: {}", Emoji("🛢️", ""), self.gas_limit)?;
//             writeln!(f, "- {}Base Fee: {}", Emoji("💸 ", ""), self.base_fee)?;
//             Ok(())
//         }
//     }
//
//     impl Display for Account {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             write!(
//                 f,
//                 "{}: {}{:?}",
//                 self.address,
//                 Emoji("💵", ""),
//                 Ether(self.balance),
//             )?;
//             write!(f, " | {}", Emoji("🔢", "Nonce: "))?;
//             write!(f, "{: >5}", self.nonce)?;
//             write!(f, " | {}", Emoji("🗄️", "Storage: "))?;
//             if self.storage.is_empty() {
//                 write!(f, "{}", style("Empty").dim())?;
//             } else {
//                 write!(f, "{: >5}", self.storage.len())?;
//             }
//
//             write!(f, " | {} ", style("</>").bold())?;
//             if self.code.is_empty() {
//                 write!(f, "{: >11}", style("Empty").dim())?;
//             } else {
//                 write!(f, "{: >5} bytes", self.code.len())?;
//             }
//             Ok(())
//         }
//     }
//
//     impl Display for Transaction {
//         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//             write!(f, "{: <7} | ", style(self.tx_type).bold().blue())?;
//             write!(f, "{} -> ", self.from)?;
//             if let Some(to) = self.to {
//                 write!(f, "{}", to)?;
//             } else {
//                 write!(f, "{}", style("CREATE").bold())?;
//             }
//             write!(f, " | {:?}", Ether(self.value))?;
//             Ok(())
//         }
//     }
// }
