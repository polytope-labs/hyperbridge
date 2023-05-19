use crate::{
    alloc::{
        format,
        string::{String, ToString},
    },
    Runtime,
};
use frame_support::PalletId;
use ismp::{
    host::StateMachine,
    module::ISMPModule,
    router::{DispatchError, DispatchResult, DispatchSuccess, ISMPRouter, Request, Response},
};
use pallet_ismp::router::ProxyRouter;

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

impl ISMPRouter for ModuleRouter {
    fn dispatch(&self, request: Request) -> DispatchResult {
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

    fn dispatch_timeout(&self, request: Request) -> DispatchResult {
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

    fn write_response(&self, response: Response) -> DispatchResult {
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

pub struct Router {
    inner: ProxyRouter<Runtime>,
}

impl Default for Router {
    fn default() -> Self {
        Self { inner: ProxyRouter::<Runtime>::new(ModuleRouter::default()) }
    }
}

impl ISMPRouter for Router {
    fn dispatch(&self, request: Request) -> DispatchResult {
        self.inner.dispatch(request)
    }

    fn dispatch_timeout(&self, request: Request) -> DispatchResult {
        self.inner.dispatch_timeout(request)
    }

    fn write_response(&self, response: Response) -> DispatchResult {
        self.inner.write_response(response)
    }
}
