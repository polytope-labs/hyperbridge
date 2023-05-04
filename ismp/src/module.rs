// Copyright (C) Polytope Labs Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! ISMPModule definition

use crate::{
    error::Error,
    router::{Request, Response},
};

pub trait ISMPModule {
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
