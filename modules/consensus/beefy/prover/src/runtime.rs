#[cfg(any(feature = "rococo-local", test))]
mod rococo_local;
#[cfg(any(feature = "rococo-local", test))]
pub use rococo_local::api::*;

#[cfg(not(feature = "rococo-local"))]
mod rococo;
#[cfg(not(feature = "rococo-local"))]
pub use rococo::api::*;
