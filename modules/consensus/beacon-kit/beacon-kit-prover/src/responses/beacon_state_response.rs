use crate::BeaconStateType;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Response<const V:usize, const L: usize> {
    pub data: BeaconStateType<V, L>
}