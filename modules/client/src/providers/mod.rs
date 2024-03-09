pub mod evm_chain;
pub mod global;
pub mod rpc_wrapper;
pub mod substrate;

pub enum StreamItem<T> {
    NoOp,
    Item(T),
}
