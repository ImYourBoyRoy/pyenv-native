// ./crates/pyenv-core/src/http.rs
//! Shared blocking HTTP client construction for pyenv-native.
//!
//! Purpose: centralize reqwest client setup so install, metadata, and self-update
//! flows use one TLS configuration policy.
//! How to use: call `build_blocking_client()` anywhere a blocking reqwest client
//! is needed.
//! Inputs: none; the helper derives the user agent from Cargo package metadata and
//! applies Android-specific TLS configuration automatically at compile time.
//! Outputs/side effects: returns a ready-to-build blocking HTTP client and, on
//! Android, avoids the platform-verifier JVM requirement by using a bundled
//! Mozilla root store with a preconfigured rustls client config.
//! Notes: Android CLI binaries such as Termux do not provide the app/JVM
//! initialization required by rustls-platform-verifier, so this module uses a
//! rustls+webpki configuration there instead of the reqwest default rustls path.

use reqwest::blocking::{Client, ClientBuilder};

pub(crate) fn build_blocking_client() -> Result<Client, reqwest::Error> {
    blocking_client_builder().build()
}

pub(crate) fn blocking_client_builder() -> ClientBuilder {
    configure_platform_tls(Client::builder()).user_agent(user_agent())
}

fn user_agent() -> String {
    format!("pyenv-native/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(target_os = "android")]
fn configure_platform_tls(builder: ClientBuilder) -> ClientBuilder {
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    builder.tls_backend_preconfigured(tls)
}

#[cfg(not(target_os = "android"))]
fn configure_platform_tls(builder: ClientBuilder) -> ClientBuilder {
    builder
}

#[cfg(test)]
mod tests {
    use super::build_blocking_client;

    #[test]
    fn blocking_client_builds() {
        build_blocking_client().expect("blocking client");
    }
}
