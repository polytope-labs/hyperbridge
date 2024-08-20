// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
