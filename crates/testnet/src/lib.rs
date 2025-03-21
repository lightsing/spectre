#[macro_use]
extern crate tracing;

use alloy_genesis::CliqueConfig;
pub use alloy_genesis::Genesis;
pub use alloy_signer::k256::ecdsa::SigningKey;
use alloy_signer::utils::secret_key_to_address;
use rand::{SeedableRng, rngs::StdRng};
use std::{
    fmt::Debug,
    fs::File,
    io,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use tempfile::TempDir;
use url::Url;

#[derive(Debug)]
pub enum TestNetBuilderError {
    GenesisNotSet,

    GethPathNotSet,
    GethPathDoesNotExist,

    FailedToCreateTempDir(io::Error),
    FailedToWriteFile(io::Error),
    FailedToWriteKeystore(eth_keystore::KeystoreError),
    Serialization(serde_json::Error),

    FailedInit,
}

pub struct TestNetBuilder {
    genesis: Option<Genesis>,
    signing_key: Option<SigningKey>,
    geth_path: Option<PathBuf>,
    /// set random_seed for deterministic tests
    random_seed: Option<u64>,
}

impl TestNetBuilder {
    #[instrument(skip(self))]
    pub fn build(self) -> Result<TestNetProvider, TestNetBuilderError> {
        use TestNetBuilderError::*;
        // for deterministic tests
        let mut rng = if let Some(random_seed) = self.random_seed {
            StdRng::seed_from_u64(random_seed)
        } else {
            StdRng::from_entropy()
        };

        let geth_path = self.geth_path.ok_or(GethPathNotSet)?;
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
            period: Some(3),
            epoch: Some(30000),
        });
        genesis.extra_data = extra_data_bytes.into();

        let temp_dir = tempfile::tempdir().map_err(FailedToCreateTempDir)?;
        trace!(temp_dir = ?temp_dir.path().display());

        let geth_data_dir = temp_dir.path().join("data");
        let keystore_dir = geth_data_dir.join("keystore");
        let password_file = temp_dir.path().join("password");
        std::fs::create_dir_all(&keystore_dir).map_err(FailedToCreateTempDir)?;
        // write password
        std::fs::write(&password_file, "testnet").map_err(FailedToWriteFile)?;
        trace!(password_file = ?password_file);

        // write genesis.json
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
            "testnet",
            None,
        )
        .map_err(FailedToWriteKeystore)?;
        trace!(keystore_dir = ?keystore_dir.display(), keystore_name = ?keystore_name);

        // execute geth init
        let geth_init_status = std::process::Command::new(&geth_path)
            .arg("--datadir")
            .arg(&geth_data_dir)
            .arg("init")
            .arg(geth_data_dir.join("genesis.json"))
            .output()
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
        let mut child = std::process::Command::new(&geth_path)
            .args([
                "--port=0",
                "--nodiscover",
                "--nat=none",
                "--syncmode=full",
                "--verbosity=3",
                "--http",
                "--http.port=0",
                "--http.addr=127.0.0.1",
                "--http.vhosts=*",
                "--http.corsdomain=*",
                "--http.api=eth,scroll,net,web3,debug,clique",
                "--allow-insecure-unlock",
                "--mine",
            ])
            .arg("--datadir")
            .arg(geth_data_dir)
            .arg("--unlock")
            .arg(signer_addr.to_string())
            .arg("--password")
            .arg(password_file)
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                error!("Failed to run geth: {}", e);
                FailedInit
            })?;

        let mut stderr = child.stderr.take().unwrap();
        let url = {
            let mut stderr = BufReader::new(&mut stderr).lines();
            'outer: loop {
                let Some(Ok(line)) = stderr.next() else {
                    break None;
                };
                debug!("{}", line);
                if line.contains("HTTP server started") {
                    for item in line.split_whitespace() {
                        if item.starts_with("endpoint=") {
                            let host = item.split('=').last().unwrap();
                            break 'outer Some(Url::parse(&*format!("http://{host}")).unwrap());
                        }
                    }
                }
            }
            .ok_or(FailedInit)?
        };
        trace!(url = ?url);
        child.stderr = Some(stderr);

        let provider = TestNetProvider {
            temp_dir,
            child,
            url,
        };
        trace!(provider = ?provider);

        Ok(provider)
    }
}

pub struct TestNetProvider {
    temp_dir: TempDir,
    child: std::process::Child,
    url: Url,
}

impl TestNetProvider {
    pub fn url(&self) -> &Url {
        &self.url
    }
}

impl Debug for TestNetProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestNetProvider")
            .field("temp_dir", &self.temp_dir.path())
            .field("child", &self.child.id())
            .field("url", &self.url.as_str())
            .finish()
    }
}

impl Drop for TestNetProvider {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")),
        )
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let builder = TestNetBuilder {
            genesis: Some(Genesis::default()),
            signing_key: None,
            geth_path: Some(PathBuf::from(
                "/Users/hhq/workspace/go-ethereum/build/bin/geth",
            )),
            random_seed: None,
        };
        builder.build().unwrap();
    }
}
