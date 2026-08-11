//! Standalone Language Server Protocol process for Yamalgam.

fn main() -> Result<(), yamalgam_lsp::ServerError> {
    yamalgam_lsp::run_stdio()
}
