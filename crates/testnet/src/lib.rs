#[macro_use]
extern crate tracing;

use alloy_genesis::{CliqueConfig, Genesis};
use alloy_provider::{IpcConnect, Provider, ProviderBuilder, RootProvider};
use alloy_signer::{k256::ecdsa::SigningKey, utils::secret_key_to_address};
use alloy_transport::TransportResult;
use rand::{SeedableRng, rngs::StdRng};
use sbv_primitives::{Address, types::Network};
use serde_json::json;
use std::{fmt::Debug, fs::File, io, path::PathBuf, sync::Arc};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

const MINER_PASSWORD: &str = "testnet";

/// Test net builder error.
#[derive(Debug, thiserror::Error)]
pub enum TestNetBuilderError {
    #[error("genesis not set")]
    GenesisNotSet,
    #[error("geth path not set and not found in path")]
    GethPathNotSet,
    #[error("geth path does not exist")]
    GethPathDoesNotExist,

    #[error("failed to create temp dir: {0}")]
    FailedToCreateTempDir(io::Error),
    #[error("failed to write file: {0}")]
    FailedToWriteFile(io::Error),
    #[error("failed to write keystore: {0}")]
    FailedToWriteKeystore(eth_keystore::KeystoreError),
    #[error("serialization error: {0}")]
    Serialization(serde_json::Error),
    #[error("failed to init geth")]
    FailedInit,
    #[error("failed to connect to geth: {0}")]
    FailedToConnectToGeth(alloy_transport::TransportError),
}

/// Test net builder.
#[derive(Default)]
pub struct TestNetBuilder<'a> {
    genesis: Option<Genesis>,
    signing_key: Option<SigningKey>,
    geth_path: Option<PathBuf>,
    rng: Option<&'a mut StdRng>,
}

impl<'a> TestNetBuilder<'a> {
    /// Set the genesis configuration.
    pub fn genesis(mut self, genesis: Genesis) -> Self {
        self.genesis = Some(genesis);
        self
    }

    /// Set the geth executable path.
    pub fn geth_path(mut self, geth_path: PathBuf) -> Self {
        self.geth_path = Some(geth_path);
        self
    }

    /// Optional, Set the signing key of the miner.
    pub fn signing_key(mut self, signing_key: SigningKey) -> Self {
        self.signing_key = Some(signing_key);
        self
    }

    /// Optional, set the random seed for deterministic behavior.
    pub fn rng(mut self, rng: &'a mut StdRng) -> Self {
        self.rng = Some(rng);
        self
    }

    /// Create the test net provider.
    #[instrument(skip(self))]
    pub async fn build(self) -> Result<TestNetProvider, TestNetBuilderError> {
        use TestNetBuilderError::*;
        // for deterministic tests
        let mut rng = if let Some(rng) = self.rng {
            rng
        } else {
            &mut StdRng::from_entropy()
        };

        let geth_path = self
            .geth_path
            .or_else(|| which::which("geth").ok())
            .ok_or(GethPathNotSet)?;
        if !geth_path.exists() || !geth_path.is_file() {
            return Err(GethPathDoesNotExist);
        }

        let mut genesis = self.genesis.ok_or(GenesisNotSet)?;

        let signing_key = self
            .signing_key
            .unwrap_or_else(|| SigningKey::random(&mut rng));
        let signer_addr = secret_key_to_address(&signing_key);

        // config clique
        let extra_data_bytes = [&[0u8; 32][..], signer_addr.as_slice(), &[0u8; 65][..]].concat();
        genesis.config.clique = Some(CliqueConfig {
            period: None,
            epoch: None,
        });
        genesis.extra_data = extra_data_bytes.into();

        let temp_dir = tempfile::tempdir().map_err(FailedToCreateTempDir)?;
        trace!(temp_dir = ?temp_dir.path().display());

        let geth_data_dir = temp_dir.path().join("data");
        let keystore_dir = geth_data_dir.join("keystore");
        // let password_file = temp_dir.path().join("password");
        std::fs::create_dir_all(&keystore_dir).map_err(FailedToCreateTempDir)?;
        // write password
        // std::fs::write(&password_file, "testnet").map_err(FailedToWriteFile)?;
        // trace!(password_file = ?password_file);

        // write genesis.json
        trace!("{}", serde_json::to_string_pretty(&genesis).unwrap());
        serde_json::to_writer_pretty(
            File::create(geth_data_dir.join("genesis.json")).map_err(FailedToWriteFile)?,
            &genesis,
        )
        .map_err(Serialization)?;
        // write keystore
        let keystore_name = eth_keystore::encrypt_key(
            &keystore_dir,
            &mut rng,
            signing_key.to_bytes(),
            MINER_PASSWORD,
            None,
        )
        .map_err(FailedToWriteKeystore)?;
        trace!(keystore_dir = ?keystore_dir.display(), keystore_name = ?keystore_name);

        // execute geth init
        let geth_init_status = Command::new(&geth_path)
            .arg("--datadir")
            .arg(&geth_data_dir)
            .arg("init")
            .arg(geth_data_dir.join("genesis.json"))
            .output()
            .await
            .map_err(|e| {
                error!("Failed to run geth init: {}", e);
                FailedInit
            })?;
        if !geth_init_status.status.success() {
            error!(
                "Failed to run geth init: \n{}",
                String::from_utf8_lossy(&geth_init_status.stderr)
            );
            return Err(FailedInit);
        }

        // execute geth
        let mut child = Command::new(&geth_path)
            .args([
                "--port=0",
                "--nodiscover",
                "--nat=none",
                "--syncmode=full",
                "--verbosity=5",
                "--txpool.globalqueue=4096",
                "--txpool.globalslots=40960",
                "--txpool.pricelimit=48700001",
                "--txpool.nolocals",
                "--miner.gaslimit=10000000",
                "--miner.gasprice=48700001",
                "--rpc.gascap=0",
                "--gpo.ignoreprice=1",
                // "--http",
                // "--http.port=0",
                // "--http.addr=127.0.0.1",
                // "--http.vhosts=*",
                // "--http.corsdomain=*",
                // "--http.api=eth,scroll,net,web3,debug,clique",
                // "--allow-insecure-unlock",
                // "--mine",
            ])
            .arg("--datadir")
            .arg(&geth_data_dir)
            // .arg("--unlock")
            // .arg(signer_addr.to_string())
            // .arg("--password")
            // .arg(password_file)
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                error!("Failed to run geth: {}", e);
                FailedInit
            })?;

        {
            let stderr = child.stderr.take().unwrap();
            let mut stderr = BufReader::new(stderr).lines();
            loop {
                let Ok(Some(line)) = stderr.next_line().await else {
                    return Err(FailedInit);
                };
                debug!(target: "geth", "{}", line);
                if line.contains("IPC endpoint opened") {
                    break;
                }
            }
            tokio::spawn(async move {
                while let Ok(Some(line)) = stderr.next_line().await {
                    debug!(target: "geth", "{}", line);
                }
            })
        };

        let ipc = IpcConnect::new(geth_data_dir.join("geth.ipc"));
        let inner = ProviderBuilder::<_, _, Network>::default()
            .on_ipc(ipc)
            .await
            .map_err(FailedToConnectToGeth)?;

        let provider = TestNetProvider(Arc::new(TestNetProviderInner {
            temp_dir,
            child: Some(child),
            inner,
            signer_addr,
        }));
        trace!(provider = ?provider);

        Ok(provider)
    }
}

/// Test net provider.
#[derive(Clone)]
pub struct TestNetProvider(Arc<TestNetProviderInner>);

struct TestNetProviderInner {
    temp_dir: TempDir,
    child: Option<tokio::process::Child>,
    inner: RootProvider<Network>,
    signer_addr: Address,
}

impl TestNetProvider {
    pub async fn stop_miner(&self) -> TransportResult<()> {
        let no_params = serde_json::value::to_raw_value(&()).unwrap();
        self.raw_request_dyn("miner_stop".into(), &no_params)
            .await?;
        Ok(())
    }

    pub async fn start_miner(&self) -> TransportResult<()> {
        let params = serde_json::value::to_raw_value(&json!([
            self.0.signer_addr.to_string(),
            MINER_PASSWORD,
            0,
        ]))
        .unwrap();
        self.raw_request_dyn("personal_unlockAccount".into(), &params)
            .await?;
        let no_params = serde_json::value::to_raw_value(&()).unwrap();
        self.raw_request_dyn("miner_start".into(), &no_params)
            .await?;
        Ok(())
    }
}

impl Debug for TestNetProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestNetProvider")
            .field("temp_dir", &self.0.temp_dir.path())
            .field("child", &self.0.child.as_ref().unwrap().id())
            .finish()
    }
}

impl Provider<Network> for TestNetProvider {
    fn root(&self) -> &RootProvider<Network> {
        &self.0.inner
    }
}
