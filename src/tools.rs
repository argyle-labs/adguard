//! AdGuard Home tool surface.
//!
//! Endpoint registry: `adguard.{list, detail, create, update, delete}` —
//! generated wholesale by `#[endpoint_resource]` (row struct, db helpers, schema
//! fragment, args/output types, and the five `#[orca_tool]` fns).
//!
//! Hand-written tools over the `/control/*` REST API:
//!   - `adguard.rewrite.list`     list DNS rewrites for an endpoint
//!   - `adguard.rewrite.add`      add one rewrite
//!   - `adguard.rewrite.delete`   delete the rewrite matching (domain, answer)
//!   - `adguard.rewrite.set`      idempotent upsert: point `domain` at `answer`
//!   - `adguard.status`           version + running/protection flags
//!
//! Imports flow through `plugin_toolkit::prelude::*` only.

use plugin_toolkit::prelude::*;

use crate::{Config, Rewrite, Status};

// ═══════════════════════════════════════════════════════════════════════════
// adguard.{list,detail,create,update,delete} — endpoint registry CRUD.
// ═══════════════════════════════════════════════════════════════════════════

// `routes` is a built-in column on every `#[endpoint_resource]` — an ordered
// fallback list (`--route kind=url`, repeatable, e.g. `--route lan=http://host:80`)
// resolved by `route::resolve_reachable`. Each entry's free-form `kind`
// (`fqdn` / `lan` / `tailscale`) doubles as the locality class the fewest-hop
// router consumes.
#[endpoint_resource(plugin = "adguard")]
pub struct AdguardEndpoint {
    pub name: String,
    pub username: String,
    #[secret]
    pub password: String,
    pub insecure: bool,
    pub enabled: bool,
}

// ── HTTP client helper ─────────────────────────────────────────────────────

/// Resolve a registered endpoint into a ready [`Config`]: the first reachable
/// base URL (`resolve_reachable` over the endpoint's `routes` fallback list)
/// plus the secure-first admin password.
pub(crate) async fn resolve_config(name: &str) -> Result<Config> {
    let row = endpoint_db::require(name)?;
    // Prefer the abstract secrets domain (`adguard.<endpoint>.password`), falling
    // back to a legacy plaintext column value only if the domain has none.
    let password = plugin_toolkit::secrets::resolve_scoped(
        "adguard",
        name,
        "password",
        (!row.password.is_empty()).then_some(row.password.as_str()),
    )?;
    let base_url = route::resolve_reachable(name, &row.routes, row.insecure).await?;
    Ok(Config::new(base_url, row.username, password).insecure(row.insecure))
}

// ═══════════════════════════════════════════════════════════════════════════
// adguard.status
// ═══════════════════════════════════════════════════════════════════════════

#[derive(clap::Args, Serialize, Deserialize, JsonSchema)]
pub struct EndpointArgs {
    /// Registered adguard endpoint name.
    #[arg(long)]
    pub name: String,
}

/// Read an AdGuard Home instance's version and running/protection status.
#[orca_tool(domain = "adguard", verb = "status", role = "any")]
async fn adguard_status(args: EndpointArgs, _ctx: &ToolCtx) -> Result<Status> {
    let cfg = resolve_config(&args.name).await?;
    let client = cfg.build_client()?;
    Ok(crate::status(&client, &cfg).await?)
}

// ═══════════════════════════════════════════════════════════════════════════
// adguard.rewrite.list
// ═══════════════════════════════════════════════════════════════════════════

/// List every DNS rewrite configured on an AdGuard Home instance.
#[orca_tool(domain = "adguard", verb = "rewrite.list", role = "any")]
async fn adguard_rewrite_list(args: EndpointArgs, _ctx: &ToolCtx) -> Result<Vec<Rewrite>> {
    let cfg = resolve_config(&args.name).await?;
    let client = cfg.build_client()?;
    Ok(crate::list_rewrites(&client, &cfg).await?)
}

// ═══════════════════════════════════════════════════════════════════════════
// adguard.rewrite.add / adguard.rewrite.delete
// ═══════════════════════════════════════════════════════════════════════════

#[derive(clap::Args, Serialize, Deserialize, JsonSchema)]
pub struct RewriteArgs {
    /// Registered adguard endpoint name.
    #[arg(long)]
    pub name: String,
    /// The domain to rewrite (e.g. `service.example.com`).
    #[arg(long)]
    pub domain: String,
    /// The answer the domain resolves to (an IP or another hostname).
    #[arg(long)]
    pub answer: String,
}

/// [MUTATES STATE] Add a DNS rewrite. AdGuard permits duplicate (domain, answer)
/// rows — use `adguard.rewrite.set` for idempotent upsert semantics.
#[orca_tool(domain = "adguard", verb = "rewrite.add", data_mutation = true)]
async fn adguard_rewrite_add(args: RewriteArgs, _ctx: &ToolCtx) -> Result<Rewrite> {
    let cfg = resolve_config(&args.name).await?;
    let client = cfg.build_client()?;
    let rewrite = Rewrite {
        domain: args.domain,
        answer: args.answer,
    };
    crate::add_rewrite(&client, &cfg, &rewrite).await?;
    Ok(rewrite)
}

/// [MUTATES STATE] Delete the DNS rewrite matching (domain, answer) exactly.
#[orca_tool(domain = "adguard", verb = "rewrite.delete", data_mutation = true)]
async fn adguard_rewrite_delete(args: RewriteArgs, _ctx: &ToolCtx) -> Result<Rewrite> {
    let cfg = resolve_config(&args.name).await?;
    let client = cfg.build_client()?;
    let rewrite = Rewrite {
        domain: args.domain,
        answer: args.answer,
    };
    crate::delete_rewrite(&client, &cfg, &rewrite).await?;
    Ok(rewrite)
}

// ═══════════════════════════════════════════════════════════════════════════
// adguard.rewrite.set — idempotent upsert
// ═══════════════════════════════════════════════════════════════════════════

/// [MUTATES STATE] Point `domain` at `answer`, dropping any existing rewrites for
/// `domain` first. Idempotent — the common "point a name at an IP" operation.
#[orca_tool(domain = "adguard", verb = "rewrite.set", data_mutation = true)]
async fn adguard_rewrite_set(args: RewriteArgs, _ctx: &ToolCtx) -> Result<Rewrite> {
    let cfg = resolve_config(&args.name).await?;
    let client = cfg.build_client()?;
    Ok(crate::set_rewrite(&client, &cfg, &args.domain, &args.answer).await?)
}
