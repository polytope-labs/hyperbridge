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

#[derive(Default)]
pub struct ProxyModule;

impl IsmpModule for ProxyModule {
    fn on_accept(&self, request: Post) -> Result<(), Error> {
        if request.dest != StateMachineProvider::get() {
            return pallet_ismp::Pallet::<Runtime>::dispatch_request(Request::Post(request))
        }

        let pallet_id = ModuleId::from_bytes(&request.to)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
        match pallet_id {
            ismp_demo::PALLET_ID =>
                ismp_demo::IsmpModuleCallback::<Runtime>::default().on_accept(request),
            _ => Err(Error::ImplementationSpecific("Destination module not found".to_string())),
        }
    }

    fn on_response(&self, response: Response) -> Result<(), Error> {
        if response.dest_chain() != StateMachineProvider::get() {
            return pallet_ismp::Pallet::<Runtime>::dispatch_response(response)
        }

        let request = &response.request();
        let from = match &request {
            Request::Post(post) => &post.from,
            Request::Get(get) => &get.from,
        };

        let pallet_id = ModuleId::from_bytes(from)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
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

        let pallet_id = ModuleId::from_bytes(from)
            .map_err(|err| Error::ImplementationSpecific(err.to_string()))?;
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
