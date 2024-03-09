use futures::{StreamExt, TryStreamExt};
use reconnecting_jsonrpsee_ws_client::{Client, SubscriptionId};
use std::ops::Deref;
use subxt::{
    error::RpcError,
    rpc::{RawValue, RpcClientT, RpcFuture, RpcSubscription},
};

pub struct ClientWrapper(pub Client);

impl Deref for ClientWrapper {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RpcClientT for ClientWrapper {
    fn request_raw<'a>(
        &'a self,
        method: &'a str,
        params: Option<Box<RawValue>>,
    ) -> RpcFuture<'a, Box<RawValue>> {
        Box::pin(async move {
            let res = self
                .0
                .request_raw(method.to_string(), params)
                .await
                .map_err(|e| RpcError::ClientError(Box::new(e)))?;
            Ok(res)
        })
    }

    fn subscribe_raw<'a>(
        &'a self,
        sub: &'a str,
        params: Option<Box<RawValue>>,
        unsub: &'a str,
    ) -> RpcFuture<'a, RpcSubscription> {
        Box::pin(async move {
            let stream = self
                .0
                .subscribe_raw(sub.to_string(), params, unsub.to_string())
                .await
                .map_err(|e| RpcError::ClientError(Box::new(e)))?;

            let id = match stream.id() {
                SubscriptionId::Str(id) => Some(id.clone().into_owned()),
                SubscriptionId::Num(id) => Some(id.to_string()),
            };

            let stream = stream.map_err(|e| RpcError::ClientError(Box::new(e))).boxed();
            Ok(RpcSubscription { stream, id })
        })
    }
}
