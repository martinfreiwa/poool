//! Thin binary entry point. All wiring lives in the `poool_backend` library
//! crate (see `src/lib.rs`) so that integration tests in `tests/` can build
//! the same router this binary uses via `poool_backend::build_platform_router`.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    poool_backend::run().await
}
