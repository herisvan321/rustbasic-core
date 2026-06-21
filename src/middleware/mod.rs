pub mod logging;
pub mod security_headers;
pub mod cors;
pub mod csrf;

use std::sync::Arc;
use std::pin::Pin;
use crate::requests::Request;
use crate::router::{Response, ErasedHandler};

pub type MiddlewareFn = Arc<
    dyn Fn(Request, Next) -> Pin<Box<dyn std::future::Future<Output = Response> + Send>>
        + Send
        + Sync,
>;

pub struct Next {
    pub(crate) chain: Arc<MiddlewareChain>,
}

impl Next {
    pub async fn run(self, req: Request) -> Response {
        self.chain.next(req).await
    }
}

pub enum MiddlewareChain {
    Next(MiddlewareFn, Arc<MiddlewareChain>),
    End(Arc<dyn ErasedHandler>),
}

impl MiddlewareChain {
    pub async fn next(self: Arc<Self>, req: Request) -> Response {
        match &*self {
            Self::Next(mw, next_chain) => {
                let next = Next { chain: next_chain.clone() };
                mw(req, next).await
            }
            Self::End(handler) => {
                handler.call(req).await
            }
        }
    }
}

pub fn from_fn<F, Fut>(mw: F) -> MiddlewareFn
where
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Response> + Send + 'static,
{
    Arc::new(move |req, next| Box::pin(mw(req, next)))
}
