//! `IsmpRouter` + `IsmpModule` for Solana — no-op shells.
//!
//! Today's `host::dispatch_incoming` CPI does the dest-program
//! notification eagerly inside `store_request_receipt`, so the module
//! returned here has no work left to do.

extern crate alloc;
use alloc::{boxed::Box, string::ToString, vec::Vec};

use ismp::{
    module::IsmpModule,
    router::{IsmpRouter, PostRequest, Response, Timeout},
};
use sp_weights::Weight;

pub struct SolanaRouter;

impl IsmpRouter for SolanaRouter {
    fn module_for_id(
        &self,
        _bytes: Vec<u8>,
    ) -> core::result::Result<Box<dyn IsmpModule>, anyhow::Error> {
        Ok(Box::new(SolanaCpiModule))
    }
}

pub struct SolanaCpiModule;

impl IsmpModule for SolanaCpiModule {
    fn on_accept(&self, _request: PostRequest) -> core::result::Result<Weight, anyhow::Error> {
        Ok(Weight::zero())
    }

    fn on_response(&self, _response: Response) -> core::result::Result<Weight, anyhow::Error> {
        Err(anyhow::anyhow!(
            "on_response: unsupported on inbound-only solana host".to_string()
        ))
    }

    fn on_timeout(&self, _request: Timeout) -> core::result::Result<Weight, anyhow::Error> {
        Err(anyhow::anyhow!(
            "on_timeout: unsupported on inbound-only solana host".to_string()
        ))
    }
}
