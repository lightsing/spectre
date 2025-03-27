#[macro_use]
extern crate tracing;

pub mod builder;
mod core;
mod utils;

pub use builder::{BuilderError, SpectreBuilder};
pub use core::Spectre;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();
}
