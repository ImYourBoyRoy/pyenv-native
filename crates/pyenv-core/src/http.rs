// ./crates/pyenv-core/src/http.rs
//! Shared blocking HTTP client construction for pyenv-native.
//!
//! Purpose: centralize reqwest client setup so install, metadata, and self-update
//! flows use one TLS configuration policy.
//! How to use: call `build_blocking_client()` anywhere a blocking reqwest client
//! is needed.
//! Inputs: none; the helper derives the user agent from Cargo package metadata.
//! Outputs/side effects: returns a ready-to-build blocking HTTP client using
//! reqwest's built-in rustls-tls configuration (webpki root store).
//! Notes: reqwest with the `rustls-tls` feature bundles webpki roots and handles
//! TLS correctly on all platforms including Android/Termux.

use reqwest::blocking::{Client, ClientBuilder};

pub(crate) fn build_blocking_client() -> Result<Client, reqwest::Error> {
    blocking_client_builder().build()
}

pub(crate) fn blocking_client_builder() -> ClientBuilder {
    Client::builder().user_agent(user_agent())
}

fn user_agent() -> String {
    format!("pyenv-native/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::build_blocking_client;

    #[test]
    fn blocking_client_builds() {
        build_blocking_client().expect("blocking client");
    }
}
