//! Tiles binary shim — the crate root lives in `lib.rs` so benches and
//! integration tests can call production functions directly.

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    tiles_tui_file_manager::run().await
}
