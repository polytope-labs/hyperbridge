use crate::{
    alloc::{
        boxed::Box,
        format,
        string::{String, ToString},
    },
    Runtime, StateMachineProvider,
};
use frame_support::pallet_prelude::Get;
use ismp::{
    error::Error,
    host::StateMachine,
    module::{DispatchError, DispatchResult, DispatchSuccess, IsmpModule},
    router::{IsmpRouter, Post, Request, Response},
};
use pallet_ismp::primitives::ModuleId;
use sp_std::prelude::*;

fn to_module_id(bytes: &[u8]) -> Result<ModuleId, &'static str> {
    codec::Decode::decode(&mut &bytes[..]).map_err(|_| "Failed to decode module id")
}

fn to_dispatch_error(
    msg: String,
    nonce: u64,
    source: StateMachine,
    dest: StateMachine,
) -> DispatchError {
    DispatchError { msg, nonce, source, dest }
}

fn to_dispatch_success(
    nonce: u64,
    source_chain: StateMachine,
    dest_chain: StateMachine,
) -> DispatchSuccess {
    DispatchSuccess { dest_chain, source_chain, nonce }
}

#[derive(Default)]
pub struct ProxyModule;

impl IsmpModule for ProxyModule {
    fn on_accept(&self, request: Post) -> DispatchResult {
        if request.dest_chain != StateMachineProvider::get() {
            return pallet_ismp::Pallet::<Runtime>::handle_request(Request::Post(request))
        }

        let dest = request.dest_chain;
        let source = request.source_chain;
        let nonce = request.nonce;
        let to = &request.to;

        let pallet_id = to_module_id(to).map_err(|e| DispatchError {
            msg: e.to_string(),
            nonce,
            source,
            dest,
        })?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_accept(request),
            _ => Err(DispatchError {
                msg: "Destination module not found".to_string(),
                nonce,
                source,
                dest,
            }),
        }
    }

    fn on_response(&self, response: Response) -> DispatchResult {
        if response.dest_chain() != StateMachineProvider::get() {
            return pallet_ismp::Pallet::<Runtime>::handle_response(response)
        }

        let request = &response.request();
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = to_module_id(from).map_err(|e| DispatchError {
            msg: e.to_string(),
            nonce,
            source,
            dest,
        })?;
        match pallet_id {
            ismp_demo::PALLET_ID => ismp_demo::IsmpModuleCallback::<Runtime>::default()
                .on_response(response)
                .map(|_| to_dispatch_success(nonce, source, dest))
                .map_err(|e| to_dispatch_error(format!("{:?}", e), nonce, source, dest)),
            _ => Err(DispatchError {
                msg: "Destination module not found".to_string(),
                nonce,
                source,
                dest,
            }),
        }
    }

    fn on_timeout(&self, request: Request) -> DispatchResult {
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();

        let pallet_id = to_module_id(from).map_err(|e| DispatchError {
            msg: e.to_string(),
            nonce,
            source,
            dest,
        })?;
        match pallet_id {
            ismp_demo::PALLET_ID => ismp_demo::IsmpModuleCallback::<Runtime>::default()
                .on_timeout(request)
                .map(|_| to_dispatch_success(nonce, source, dest))
                .map_err(|e| to_dispatch_error(format!("{:?}", e), nonce, source, dest)),
            _ => Err(DispatchError {
                msg: "Destination module not found".to_string(),
                nonce,
                source,
                dest,
            }),
        }
    }
}

#[derive(Default)]
pub struct Router;

impl IsmpRouter for Router {
    fn module_for_id(&self, _bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error> {
        Ok(Box::new(ProxyModule::default()))
    }
}
