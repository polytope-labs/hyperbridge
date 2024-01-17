use wasm_bindgen::prelude::*;



// flow 
// 1. check if the request has been be submitted on the destination chain -> ProccessComplete
// 2. check if the request has been seen by hyperbridge -> On the Hyperbridge Hub 
// 3. check is it was ever sent from the source chain -> MessageNeverLeftSource
// 4. return the progress status enum 

#[wasm_bindgen]
pub fn query_request_status() {
    todo!()
}


#[wasm_bindgen]
pub fn query_response_status() {
    todo!()
}



#[wasm_bindgen]
pub fn timeout_request() {
    todo!()
}


#[wasm_bindgen]
pub fn timeout_response() {
    todo!()
}