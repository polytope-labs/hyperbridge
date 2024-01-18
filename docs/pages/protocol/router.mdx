# ISMP Router

The router lives between the ISMP message handlers and modules, the router provides access to the ismp      
module which a request or response is designated for based on the destination module Id.

The router should not be accessible outside the host.
The interface for the ISMP Router is:

```rust
pub trait IsmpRouter {
    /// Get module handler by id
    /// Should decode the module id and return a handler to the appropriate `IsmpModule`
    /// implementation
    fn module_for_id(&self, bytes: Vec<u8>) -> Result<Box<dyn IsmpModule>, Error>;
}
```

### ISMP Modules

ISMP Modules are the applications(pallets or contracts) that initiate requests and receive responses, for an application
to be ISMP compatible it must implement a specific interface that allows the router to dispatch requests and responses
to it.

A module must also have a unique id configured in the runtime, so it can be identified by the router.

The required module interface is:

```rust
pub trait IsmpModule {
    /// Called by the ISMP router on a module, to notify module of a new request
    /// the module may choose to respond immediately, or in a later block
    fn on_accept(request: Post) -> Result<(), Error>;
    /// Called by the router on a module, to notify module of a response to a previously sent out
    /// request
    fn on_response(response: Response) -> Result<(), Error>;
    /// Called by the router on a module, to notify module of requests that were previously sent but
    /// have now timed-out
    fn on_timeout(request: Request) -> Result<(), Error>;
}
```