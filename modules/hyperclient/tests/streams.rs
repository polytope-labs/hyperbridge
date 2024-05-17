#![cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

/// Run the tests by `$ wasm-pack test --firefox --headless`

fn init_tracing() {
	console_error_panic_hook::set_once();
	let _ = tracing_wasm::try_set_as_global_default();
}

#[wasm_bindgen_test]
#[ignore]
async fn subscribe_to_request_status() -> Result<(), anyhow::Error> {
	init_tracing();
	hyperclient::testing::subscribe_to_request_status().await
}

#[wasm_bindgen_test]
#[ignore]
async fn test_timeout_request() -> Result<(), anyhow::Error> {
	init_tracing();

	hyperclient::testing::test_timeout_request().await
}
