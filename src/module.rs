use crate::{
    error::Error,
    router::{Request, Response},
};

pub trait IISMPModule {
    /// Called by the local ISMP router on a module, to notify module of a new request
    /// the module may choose to respond immediately, or in a later block
    fn on_accept(request: Request) -> Result<(), Error>;
    /// Called by the router on a module, to notify module of a response to a previously sent out
    /// request
    fn on_response(response: Response) -> Result<(), Error>;
    /// Called by the router on a module, to notify module of requests that were previously sent but
    /// have now timed-out
    fn on_timeout(request: Request) -> Result<(), Error>;
}
