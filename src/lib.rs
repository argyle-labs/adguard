//! AdGuard Home REST client.
//!
//! Drives an already-running AdGuard Home instance over its documented
//! `/control/*` REST API — DNS rewrite CRUD plus a status read. HTTP Basic auth.
//!
//! The wire surface is the toolkit's cap-backed HTTP client (`delegated-http`),
//! so every request rides orca's `http.request` capability and this plugin links
//! no reqwest/rustls. Hand-written call sites join their `/control/...` route
//! onto [`Config::base_url`].

#![allow(clippy::disallowed_types)]

pub mod tools;

use plugin_toolkit::reqwest;
use plugin_toolkit::service::{
    BoxFuture, Endpoint, Runtime, ServiceBackend, ServiceCapability, ServiceError, ServiceStatus,
    WorkloadSpec,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// adguard service backend — AdGuard Home DNS sinkhole.
///
/// Implements `ServiceBackend` so the generic `service.*` tools
/// (deploy/backup/restore/configure/status/connect/sync) drive adguard. This
/// facet is registered ALONGSIDE the `#[orca_tool]` DNS-rewrite surface in
/// [`tools`] — one binary, both facets, via the `Plugin` builder. Modeled on
/// the nfs StorageBackend. See orca/docs/PLUGIN-PROGRAM.md.
///
/// Holds only the provider name; per-instance endpoint/creds come from the
/// `Endpoint` the generic `service.*` tools hand each op.
#[derive(Debug, Clone)]
pub struct AdguardBackend {
    provider: &'static str,
}

impl AdguardBackend {
    pub fn new(provider: &'static str) -> Self {
        Self { provider }
    }
}

impl ServiceBackend for AdguardBackend {
    fn provider(&self) -> &str {
        self.provider
    }

    /// Runtimes adguard can be placed on. `service.deploy` hands the
    /// `workload_spec` below to a matching deploy target — this backend never
    /// drives pct/docker itself (that mechanic lives in the deploy-target domain).
    fn runtimes(&self) -> Vec<Runtime> {
        vec![Runtime::Docker, Runtime::Podman, Runtime::Lxc, Runtime::Vm]
    }

    fn capabilities(&self) -> Vec<ServiceCapability> {
        vec![
            ServiceCapability::Deploy,
            ServiceCapability::Backup,
            ServiceCapability::Restore,
            ServiceCapability::Configure,
            ServiceCapability::Status,
        ]
    }

    fn default_port(&self) -> u16 {
        3000
    }

    /// In-workload paths holding config/data. This is ALL adguard declares for
    /// backup — the generic pluggable backup (tar for containers/LXC, PBS for
    /// Proxmox guests when available) snapshots these. No backup/restore code
    /// here; those are inherited from ServiceBackend's defaults.
    fn data_paths(&self) -> Vec<String> {
        vec!["/config".to_string()]
    }

    fn workload_spec<'a>(
        &'a self,
        _runtime: Runtime,
        _ep: &'a Endpoint,
    ) -> BoxFuture<'a, Result<WorkloadSpec, ServiceError>> {
        // TODO: describe the adguard workload (image/template, ports, mounts,
        // env) for the chosen runtime. The deploy target turns this into a
        // compose service / LXC config / VM. See deploy-target::WorkloadSpec.
        Box::pin(async move { Err(ServiceError::unimplemented("adguard.workload_spec")) })
    }

    fn configure<'a>(
        &'a self,
        _ep: &'a Endpoint,
        _config: &'a str,
    ) -> BoxFuture<'a, Result<(), ServiceError>> {
        // TODO: apply adguard-specific config idempotently.
        Box::pin(async move { Err(ServiceError::unimplemented("adguard.configure")) })
    }

    fn status<'a>(
        &'a self,
        _ep: &'a Endpoint,
    ) -> BoxFuture<'a, Result<ServiceStatus, ServiceError>> {
        // TODO: real health/diagnostics.
        Box::pin(async move { Err(ServiceError::unimplemented("adguard.status")) })
    }
}

/// A single DNS rewrite: `domain` → `answer` (an IP or another hostname).
/// Matches AdGuard's `/control/rewrite/*` request/response body verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Rewrite {
    pub domain: String,
    pub answer: String,
}

/// The subset of `/control/status` this plugin surfaces.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Status {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protection_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dns_addresses: Vec<String>,
}

#[derive(Debug, Error)]
pub enum AdguardError {
    #[error("adguard transport: {0}")]
    Transport(String),
    #[error("adguard api error (status {status}): {body}")]
    Api { status: u16, body: String },
    #[error("malformed adguard response: {0}")]
    Malformed(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Base URL of the AdGuard admin interface (e.g. `http://host:80`). The
    /// `/control/...` API path is joined onto this.
    pub base_url: String,
    pub username: String,
    pub password: String,
    /// Skip TLS verification (self-signed homelab certs on an https front-end).
    pub insecure: bool,
}

impl Config {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            username: username.into(),
            password: password.into(),
            insecure: false,
        }
    }

    pub fn insecure(mut self, on: bool) -> Self {
        self.insecure = on;
        self
    }

    /// `Basic <base64(user:pass)>` — AdGuard Home's admin auth scheme.
    fn auth_header_value(&self) -> String {
        let raw = format!("{}:{}", self.username, self.password);
        format!("Basic {}", base64_encode(raw.as_bytes()))
    }

    /// Build the cap-backed HTTP client with the Basic auth header pre-attached
    /// and TLS verification toggled per `insecure`.
    pub fn build_client(&self) -> Result<reqwest::Client, AdguardError> {
        plugin_toolkit::api_client::ApiClientBuilder::new()
            .header("authorization", self.auth_header_value())
            .and_then(|b| b.insecure(self.insecure).build())
            .map_err(|e| AdguardError::Transport(format!("client build: {e}")))
    }

    /// Join a `/control/<path>` API route onto the base URL.
    fn control_url(&self, path: &str) -> String {
        format!(
            "{}/control/{}",
            self.base_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

fn transport(e: impl std::fmt::Display) -> AdguardError {
    AdguardError::Transport(e.to_string())
}

/// `GET /control/status` — version + running/protection flags.
pub async fn status(client: &reqwest::Client, cfg: &Config) -> Result<Status, AdguardError> {
    let resp = client
        .get(cfg.control_url("status"))
        .send()
        .await
        .map_err(transport)?;
    let status = resp.status();
    let body = resp.text().await.map_err(transport)?;
    if !status.is_success() {
        return Err(AdguardError::Api {
            status: status.as_u16(),
            body,
        });
    }
    plugin_toolkit::serde_json::from_str(&body).map_err(|e| AdguardError::Malformed(e.to_string()))
}

/// `GET /control/rewrite/list` — every configured DNS rewrite.
pub async fn list_rewrites(
    client: &reqwest::Client,
    cfg: &Config,
) -> Result<Vec<Rewrite>, AdguardError> {
    let resp = client
        .get(cfg.control_url("rewrite/list"))
        .send()
        .await
        .map_err(transport)?;
    let status = resp.status();
    let body = resp.text().await.map_err(transport)?;
    if !status.is_success() {
        return Err(AdguardError::Api {
            status: status.as_u16(),
            body,
        });
    }
    plugin_toolkit::serde_json::from_str(&body).map_err(|e| AdguardError::Malformed(e.to_string()))
}

/// `POST /control/rewrite/add` — add one rewrite. AdGuard allows duplicate
/// (domain, answer) rows, so callers that want upsert semantics should use
/// [`set_rewrite`].
pub async fn add_rewrite(
    client: &reqwest::Client,
    cfg: &Config,
    rewrite: &Rewrite,
) -> Result<(), AdguardError> {
    post_ok(client, &cfg.control_url("rewrite/add"), rewrite).await
}

/// `POST /control/rewrite/delete` — remove the rewrite matching (domain, answer)
/// exactly.
pub async fn delete_rewrite(
    client: &reqwest::Client,
    cfg: &Config,
    rewrite: &Rewrite,
) -> Result<(), AdguardError> {
    post_ok(client, &cfg.control_url("rewrite/delete"), rewrite).await
}

/// Idempotent upsert for a domain: drop every existing rewrite for `domain`
/// (any answer), then add the desired one. AdGuard has no direct update verb, so
/// this is the delete-then-add pattern. Returns the applied rewrite.
pub async fn set_rewrite(
    client: &reqwest::Client,
    cfg: &Config,
    domain: &str,
    answer: &str,
) -> Result<Rewrite, AdguardError> {
    let existing = list_rewrites(client, cfg).await?;
    for r in existing.iter().filter(|r| r.domain == domain) {
        delete_rewrite(client, cfg, r).await?;
    }
    let desired = Rewrite {
        domain: domain.to_string(),
        answer: answer.to_string(),
    };
    add_rewrite(client, cfg, &desired).await?;
    Ok(desired)
}

/// POST a JSON body and treat any 2xx as success (AdGuard's write endpoints
/// answer 200 with an empty/uninteresting body).
async fn post_ok<B: Serialize>(
    client: &reqwest::Client,
    url: &str,
    body: &B,
) -> Result<(), AdguardError> {
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(transport)?;
    let status = resp.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(AdguardError::Api {
            status: status.as_u16(),
            body,
        })
    }
}

/// Standard-alphabet base64 for the Basic auth header. Hand-rolled to keep the
/// dependency set minimal (Basic auth is the only base64 use in the plugin).
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18 & 0x3f) as usize] as char);
        out.push(ALPHABET[(n >> 12 & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6 & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn auth_header_is_basic_scheme() {
        let cfg = Config::new("http://host", "admin", "s3cret");
        // base64("admin:s3cret")
        assert_eq!(cfg.auth_header_value(), "Basic YWRtaW46czNjcmV0");
    }

    #[test]
    fn declares_provider() {
        let b = AdguardBackend::new("adguard");
        assert_eq!(b.provider(), "adguard");
    }

    #[test]
    fn control_url_joins_cleanly() {
        let cfg = Config::new("http://host:80/", "u", "p");
        assert_eq!(
            cfg.control_url("rewrite/list"),
            "http://host:80/control/rewrite/list"
        );
        assert_eq!(cfg.control_url("/status"), "http://host:80/control/status");
    }
}
