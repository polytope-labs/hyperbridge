pub mod evm;
pub mod interface;
pub mod substrate;

pub enum StreamItem<T> {
    NoOp,
    Item(T),
}
