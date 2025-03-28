use alloy_consensus::SignableTransaction;
#[cfg(not(feature = "scroll"))]
use alloy_consensus::{TxEip4844Variant, TxEnvelope, TypedTransaction};
use alloy_genesis::Genesis;
use alloy_network::{ReceiptResponse, TxSignerSync};
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_signer_local::PrivateKeySigner;
use sbv_primitives::types::BlockWitness;
use sbv_utils::rpc::ProviderExt;
#[cfg(feature = "scroll")]
use scroll_alloy_consensus::{
    ScrollTxEnvelope as TxEnvelope, ScrollTypedTransaction as TypedTransaction,
};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    path::PathBuf,
};

#[derive(Debug, thiserror::Error)]
pub enum SpectreError {
    #[error("Error while build testnet: {0}")]
    TestnetBuilder(#[from] testnet::TestNetBuilderError),
    #[error("Error while serializing/deserializing JSON: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Error while sending rpc request: {0}")]
    Rpc(#[from] alloy_json_rpc::RpcError<alloy_transport::TransportErrorKind>),
    #[error("Error while waiting for receipt: {0}")]
    PendingTransaction(#[from] alloy_provider::PendingTransactionError),
}

// #[derive(Debug)]
pub struct Spectre {
    pub(crate) geth_path: Option<PathBuf>,
    pub(crate) genesis: Genesis,
    pub(crate) wallets: HashMap<Address, PrivateKeySigner>,
    pub(crate) transactions: Vec<(Address, TypedTransaction)>,
}

impl Spectre {
    pub async fn trace(self) -> Result<Vec<BlockWitness>, SpectreError> {
        let mut provider_builder = testnet::TestNetBuilder::default();
        if let Some(geth_path) = self.geth_path {
            provider_builder = provider_builder.geth_path(geth_path);
        }
        let provider = provider_builder.genesis(self.genesis).build().await?;
        provider.stop_miner().await?;

        let mut nonce_map = HashMap::new();

        let mut txs = vec![];
        for (from, tx) in self.transactions.into_iter() {
            if !nonce_map.contains_key(&from) {
                let nonce = provider.get_transaction_count(from).await?;
                nonce_map.insert(from, nonce);
            }
            let nonce = {
                let nonce = nonce_map.get_mut(&from).unwrap();
                let current = *nonce;
                *nonce += 1;
                current
            };

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
            let mut pending_tx = provider.send_tx_envelope(tx_envelope).await?;
            pending_tx.set_required_confirmations(0);
            txs.push(pending_tx);
        }

        provider.start_miner().await?;

        let mut blocks = BTreeSet::new();
        let mut witnesses = vec![];
        for tx in txs {
            let receipt = tx.get_receipt().await?;
            let block_number = receipt.block_number().unwrap();
            if blocks.contains(&block_number) {
                continue;
            }

            let witness = provider
                .dump_block_witness(block_number.into())
                .await?
                .unwrap();
            blocks.insert(block_number);
            witnesses.push(witness);
        }
        trace!(witnesses = %witnesses.len());

        Ok(witnesses)
    }
}

#[cfg(feature = "cli")]
mod display {
    use super::*;
    use crate::utils::*;
    use alloy_consensus::Transaction;
    use alloy_genesis::GenesisAccount;
    use console::{Emoji, style};
    use std::fmt::Display;

    struct DisplayAccount<'a> {
        addr: &'a Address,
        acc: &'a GenesisAccount,
    }
    struct DisplayTransaction<'a> {
        from: &'a Address,
        typed_tx: &'a TypedTransaction,
    }

    impl Display for Spectre {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(
                f,
                "{}{}",
                style("Loaded Spectre").bold().blue(),
                Emoji(" 👻", "")
            )?;
            writeln!(
                f,
                "{} {} genesis accounts:",
                Emoji("💳", ""),
                self.genesis.alloc.len()
            )?;
            for (address, account) in self.genesis.alloc.iter() {
                if !self.wallets.contains_key(address) {
                    write!(f, "{} ", Emoji("- 👤", "- [address]"),)?;
                } else {
                    write!(f, "{} ", Emoji("- 🔐", "- [ wallet]"),)?;
                }
                writeln!(f, "{}", DisplayAccount {
                    addr: address,
                    acc: account
                })?;
            }
            writeln!(
                f,
                "\n{} {} transactions:",
                Emoji("💸", ""),
                self.transactions.len()
            )?;
            for (from, tx) in &self.transactions {
                writeln!(f, "- {}", DisplayTransaction { from, typed_tx: tx })?;
            }
            Ok(())
        }
    }

    impl Display for DisplayAccount<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}: {}{:?}",
                self.addr,
                Emoji("💵", ""),
                Ether(self.acc.balance),
            )?;
            write!(f, " | {}", Emoji("🔢", "Nonce: "))?;
            write!(f, "{: >5}", self.acc.nonce.unwrap_or_default())?;
            write!(f, " | {}", Emoji("🗄️", "Storage: "))?;
            let storage_len = self
                .acc
                .storage
                .as_ref()
                .map(|s| s.len())
                .unwrap_or_default();
            if storage_len == 0 {
                write!(f, "{}", style("Empty").dim())?;
            } else {
                write!(f, "{: >5}", storage_len)?;
            }

            write!(f, " | {} ", style("</>").bold())?;
            let code_len = self.acc.code.as_ref().map(|s| s.len()).unwrap_or_default();
            if code_len == 0 {
                write!(f, "{: >11}", style("Empty").dim())?;
            } else {
                write!(f, "{: >5} bytes", code_len)?;
            }
            Ok(())
        }
    }
    impl Display for DisplayTransaction<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{: <7} | ", style(self.typed_tx.tx_type()).bold().blue())?;
            write!(f, "{} -> ", self.from)?;
            if let Some(to) = self.typed_tx.to() {
                write!(f, "{}", to)?;
            } else {
                write!(f, "{}", style("CREATE").bold())?;
            }
            write!(f, " | {:?}", Ether(self.typed_tx.value()))?;
            Ok(())
        }
    }
}
