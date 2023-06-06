use crate::{
    alloc::{
        format,
        string::{String, ToString},
    },
    Runtime, StateMachineProvider,
};
use frame_support::{pallet_prelude::Get, PalletId};
use ismp::{
    host::StateMachine,
    module::IsmpModule,
    router::{DispatchError, DispatchResult, DispatchSuccess, IsmpRouter, Request, Response},
};

fn to_pallet_id(bytes: &[u8]) -> Result<PalletId, &'static str> {
    if bytes.len() != 8 {
        Err("Invalid pallet id")?
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bytes);
    Ok(PalletId(buf))
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
pub struct ModuleRouter;

impl IsmpRouter for ModuleRouter {
    fn handle_request(&self, request: Request) -> DispatchResult {
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();
        let to = match &request {
            Request::Post(post) => &post.to,
            Request::Get(get) => &get.from,
        };

        let pallet_id = to_pallet_id(to).map_err(|e| DispatchError {
            msg: e.to_string(),
            nonce,
            source,
            dest,
        })?;
        match pallet_id {
            ismp_assets::PALLET_ID => ismp_assets::Pallet::<Runtime>::on_accept(request)
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

    fn handle_timeout(&self, request: Request) -> DispatchResult {
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();

        let pallet_id = to_pallet_id(from).map_err(|e| DispatchError {
            msg: e.to_string(),
            nonce,
            source,
            dest,
        })?;
        match pallet_id {
            ismp_assets::PALLET_ID => ismp_assets::Pallet::<Runtime>::on_timeout(request)
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

    fn handle_response(&self, response: Response) -> DispatchResult {
        let request = &response.request();
        let dest = request.dest_chain();
        let source = request.source_chain();
        let nonce = request.nonce();
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = to_pallet_id(from).map_err(|e| DispatchError {
            msg: e.to_string(),
            nonce,
            source,
            dest,
        })?;
        match pallet_id {
            ismp_assets::PALLET_ID => ismp_assets::Pallet::<Runtime>::on_response(response)
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
pub struct ProxyRouter {
    inner: ModuleRouter,
}

impl IsmpRouter for ProxyRouter {
    fn handle_request(&self, request: Request) -> DispatchResult {
        if request.dest_chain() != StateMachineProvider::get() {
            pallet_ismp::Pallet::<Runtime>::handle_request(request)
        } else {
            self.inner.handle_request(request)
        }
    }

    fn handle_timeout(&self, request: Request) -> DispatchResult {
        self.inner.handle_timeout(request)
    }

    fn handle_response(&self, response: Response) -> DispatchResult {
        if response.dest_chain() != StateMachineProvider::get() {
            pallet_ismp::Pallet::<Runtime>::handle_response(response)
        } else {
            self.inner.handle_response(response)
        }
    }
}
