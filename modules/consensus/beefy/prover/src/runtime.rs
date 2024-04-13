#[cfg(feature = "rococo-local")]
mod rococo_local;
#[cfg(feature = "rococo-local")]
pub use rococo_local::api::*;

#[cfg(not(feature = "rococo-local"))]
mod rococo;
#[cfg(not(feature = "rococo-local"))]
pub use rococo::api::*;
