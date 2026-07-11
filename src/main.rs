//! Dynamic (subprocess) entrypoint for the adguard plugin.
//!
//! The toolkit's `serve_service_plugin!` emits `fn main`, serving this plugin over the orca
//! socket. Dynamic replacement for the retired cdylib export — the plugin is a
//! `[[bin]]`, owns no runtime, and reaches orca only through the socket.
plugin_toolkit::serve_service_plugin! {
    name: "adguard",
    target_compat: "any",
    backend: adguard::AdguardBackend::new("adguard"),
}
