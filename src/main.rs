//! Dynamic (subprocess) entrypoint for the adguard plugin.
//!
//! A DUAL-facet plugin: one `Plugin` builder chain registers BOTH the
//! [`ServiceBackend`](adguard::AdguardBackend) (generic `service.*` lifecycle)
//! AND the `adguard.` `#[orca_tool]` surface (endpoint registry CRUD + DNS
//! rewrite CRUD + status). The builder emits all the wire dispatch, so the
//! plugin hand-writes no op strings and owns no runtime — it reaches orca only
//! through the socket.
plugin_toolkit::instrument::bootstrap!();

use adguard::AdguardBackend;
use plugin_toolkit::plugin::Plugin;

// Force-link this plugin's OWN lib crate so the linker doesn't dead-strip the
// rlib (and with it every `#[orca_tool]` / `#[endpoint_resource]` registration).
// The builder does NOT force-link for you, so this `use ... as _;` is required.
#[allow(unused_imports)]
use adguard::tools as _;

fn main() -> plugin_toolkit::anyhow::Result<()> {
    Plugin::named("adguard")
        .version(env!("CARGO_PKG_VERSION"))
        .service(AdguardBackend::new("adguard"))
        .tools(["adguard."])
        .serve()
}
