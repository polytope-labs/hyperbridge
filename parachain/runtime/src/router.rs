use crate::{
    alloc::{boxed::Box, string::ToString},
    Runtime, StateMachineProvider,
};
use frame_support::pallet_prelude::Get;
use ismp::{
    error::Error,
    module::IsmpModule,
    router::{IsmpRouter, Post, Request, Response},
};
use pallet_ismp::primitives::ModuleId;
use sp_std::prelude::*;

fn to_module_id(bytes: &[u8]) -> Result<ModuleId, Error> {
    codec::Decode::decode(&mut &bytes[..])
        .map_err(|_| Error::ImplementationSpecific("Failed to decode module id".to_string()))
}

#[derive(Default)]
pub struct ProxyModule;

impl IsmpModule for ProxyModule {
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        if request.source_chain == StateMachineProvider::get() {
            // lol, you really didn't think it would be that easy?
            Err(Error::CannotHandleMessage)?
        }

        if request.dest_chain != StateMachineProvider::get() {
            return pallet_ismp::Pallet::<Runtime>::handle_request(Request::Post(request))
        }

        let to = &request.to;

        let pallet_id = to_module_id(to)?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_accept(request),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
        }
    }

    fn on_response(&self, response: Response) -> Result<(), Error> {
        if response.dest_chain() != StateMachineProvider::get() {
            return pallet_ismp::Pallet::<Runtime>::handle_response(response)
        }

        let request = &response.request();
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = to_module_id(from)?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_response(response),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
        }
    }

    fn on_timeout(&self, request: Request) -> Result<(), Error> {
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = to_module_id(from)?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_timeout(request),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
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
