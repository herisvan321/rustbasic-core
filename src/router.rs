use std::sync::Arc;
use crate::requests::Request;


pub type Response = http::Response<Vec<u8>>;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(self.as_bytes().to_vec())
            .unwrap()
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .body(self.into_bytes())
            .unwrap()
    }
}

pub struct Html<T>(pub T);
impl<T: Into<String>> IntoResponse for Html<T> {
    fn into_response(self) -> Response {
        http::Response::builder()
            .header(http::header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.0.into().into_bytes())
            .unwrap()
    }
}

pub struct Json<T>(pub T);
impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        let body = serde_json::to_vec(&self.0).unwrap_or_default();
        http::Response::builder()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(body)
            .unwrap()
    }
}

pub struct Redirect {
    url: String,
}
impl Redirect {
    pub fn to(url: &str) -> Self {
        Self { url: url.to_string() }
    }
}
impl IntoResponse for Redirect {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(http::StatusCode::SEE_OTHER)
            .header(http::header::LOCATION, &self.url)
            .body(Vec::new())
            .unwrap()
    }
}

impl IntoResponse for serde_json::Value {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl IntoResponse for http::StatusCode {
    fn into_response(self) -> Response {
        http::Response::builder()
            .status(self)
            .body(Vec::new())
            .unwrap()
    }
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(r) => r.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

impl<T: IntoResponse> IntoResponse for (http::StatusCode, T) {
    fn into_response(self) -> Response {
        let mut res = self.1.into_response();
        *res.status_mut() = self.0;
        res
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct State<T>(pub T);

#[crate::async_trait]
pub trait Handler<T>: Send + Sync + 'static {
    async fn call(&self, req: Request) -> Response;
}

#[crate::async_trait]
pub trait ErasedHandler: Send + Sync + 'static {
    async fn call(&self, req: Request) -> Response;
}

pub struct HandlerWrapper<H, T> {
    pub(crate) handler: H,
    pub(crate) _marker: std::marker::PhantomData<T>,
}

#[crate::async_trait]
impl<H, T> ErasedHandler for HandlerWrapper<H, T>
where
    H: Handler<T>,
    T: Send + Sync + 'static,
{
    async fn call(&self, req: Request) -> Response {
        self.handler.call(req).await
    }
}

// Arity 0: fn() -> R
#[crate::async_trait]
impl<F, Fut, R> Handler<()> for F
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = R> + Send + 'static,
    R: IntoResponse + Send + 'static,
{
    async fn call(&self, _req: Request) -> Response {
        self().await.into_response()
    }
}

// Arity 1: fn(Request) -> R
#[crate::async_trait]
impl<F, Fut, R> Handler<(Request,)> for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = R> + Send + 'static,
    R: IntoResponse + Send + 'static,
{
    async fn call(&self, req: Request) -> Response {
        self(req).await.into_response()
    }
}

// Arity 2: fn(State<AppState>, Request) -> R
#[crate::async_trait]
impl<F, Fut, R> Handler<(State<crate::AppState>, Request)> for F
where
    F: Fn(State<crate::AppState>, Request) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = R> + Send + 'static,
    R: IntoResponse + Send + 'static,
{
    async fn call(&self, req: Request) -> Response {
        let state = State(req.state.clone());
        self(state, req).await.into_response()
    }
}

#[derive(Clone)]
pub struct MethodRouter {
    pub(crate) handlers: Vec<(http::Method, Arc<dyn ErasedHandler>)>,
}

pub fn get<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: Send + Sync + 'static,
{
    let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
        handler,
        _marker: std::marker::PhantomData,
    });
    MethodRouter {
        handlers: vec![(http::Method::GET, wrapped)],
    }
}

pub fn post<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: Send + Sync + 'static,
{
    let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
        handler,
        _marker: std::marker::PhantomData,
    });
    MethodRouter {
        handlers: vec![(http::Method::POST, wrapped)],
    }
}

pub fn put<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: Send + Sync + 'static,
{
    let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
        handler,
        _marker: std::marker::PhantomData,
    });
    MethodRouter {
        handlers: vec![(http::Method::PUT, wrapped)],
    }
}

pub fn patch<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: Send + Sync + 'static,
{
    let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
        handler,
        _marker: std::marker::PhantomData,
    });
    MethodRouter {
        handlers: vec![(http::Method::PATCH, wrapped)],
    }
}

pub fn delete<H, T>(handler: H) -> MethodRouter
where
    H: Handler<T>,
    T: Send + Sync + 'static,
{
    let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
        handler,
        _marker: std::marker::PhantomData,
    });
    MethodRouter {
        handlers: vec![(http::Method::DELETE, wrapped)],
    }
}

impl MethodRouter {
    pub fn get<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T>,
        T: Send + Sync + 'static,
    {
        let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
            handler,
            _marker: std::marker::PhantomData,
        });
        self.handlers.push((http::Method::GET, wrapped));
        self
    }
    
    pub fn post<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T>,
        T: Send + Sync + 'static,
    {
        let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
            handler,
            _marker: std::marker::PhantomData,
        });
        self.handlers.push((http::Method::POST, wrapped));
        self
    }
    
    pub fn put<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T>,
        T: Send + Sync + 'static,
    {
        let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
            handler,
            _marker: std::marker::PhantomData,
        });
        self.handlers.push((http::Method::PUT, wrapped));
        self
    }

    pub fn patch<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T>,
        T: Send + Sync + 'static,
    {
        let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
            handler,
            _marker: std::marker::PhantomData,
        });
        self.handlers.push((http::Method::PATCH, wrapped));
        self
    }

    pub fn delete<H, T>(mut self, handler: H) -> Self
    where
        H: Handler<T>,
        T: Send + Sync + 'static,
    {
        let wrapped: Arc<dyn ErasedHandler> = Arc::new(HandlerWrapper {
            handler,
            _marker: std::marker::PhantomData,
        });
        self.handlers.push((http::Method::DELETE, wrapped));
        self
    }
}

#[derive(Clone)]
pub struct Router<S = ()> {
    pub(crate) routes: Vec<Arc<Route>>,
    pub(crate) middlewares: Vec<crate::middleware::MiddlewareFn>,
    pub(crate) _marker: std::marker::PhantomData<fn() -> S>,
}

pub struct Route {
    pub path: String,
    pub handlers: Vec<(http::Method, Arc<dyn ErasedHandler>)>,
    pub name: Option<String>,
}

pub static NAMED_ROUTES: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();

pub fn get_named_routes() -> std::collections::HashMap<String, String> {
    NAMED_ROUTES.get().cloned().unwrap_or_default()
}


struct MiddlewareHandler {
    mw: crate::middleware::MiddlewareFn,
    next: Arc<dyn ErasedHandler>,
}

#[crate::async_trait]
impl ErasedHandler for MiddlewareHandler {
    async fn call(&self, req: Request) -> Response {
        let chain = Arc::new(crate::middleware::MiddlewareChain::End(self.next.clone()));
        let next = crate::middleware::Next { chain };
        (self.mw)(req, next).await
    }
}

impl<S> Default for Router<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Router<S> {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            middlewares: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        self.routes.push(Arc::new(Route {
            path: path.to_string(),
            handlers: method_router.handlers,
            name: None,
        }));
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        if let Some(route) = self.routes.last_mut() {
            let new_route = Route {
                path: route.path.clone(),
                handlers: route.handlers.clone(),
                name: Some(name.to_string()),
            };
            *route = Arc::new(new_route);
        }
        self
    }

    pub fn get_json<T>(self, path: &str, data: T) -> Self
    where
        T: serde::Serialize + Clone + Send + Sync + 'static,
    {
        let data_clone = data.clone();
        self.route(path, get(move || {
            let data = data_clone.clone();
            async move {
                Json(data)
            }
        }))
    }

    pub fn get_redirect(self, path: &str, to_url: &str) -> Self {
        let to_url = to_url.to_string();
        self.route(path, get(move || {
            let to = to_url.clone();
            async move {
                Redirect::to(&to)
            }
        }))
    }

    pub fn get_view(self, path: &str, template: &'static str, context: serde_json::Value) -> Self {
        let context_clone = context.clone();
        self.route(path, get(move |req: Request| {
            let ctx = context_clone.clone();
            async move {
                crate::view::view(&req, template, ctx)
            }
        }))
    }

    pub fn merge(mut self, other: Router<S>) -> Self {
        for other_route in other.routes {
            let mut handlers_with_mw = Vec::new();
            for (method, handler) in &other_route.handlers {
                let mut current_handler = handler.clone();
                for mw in other.middlewares.iter().rev() {
                    let next_handler = current_handler.clone();
                    let mw_clone = mw.clone();
                    current_handler = Arc::new(MiddlewareHandler {
                        mw: mw_clone,
                        next: next_handler,
                    });
                }
                handlers_with_mw.push((method.clone(), current_handler));
            }
            self.routes.push(Arc::new(Route {
                path: other_route.path.clone(),
                handlers: handlers_with_mw,
                name: other_route.name.clone(),
            }));
        }
        self
    }

    pub fn nest(mut self, prefix: &str, other: Router<S>) -> Self {
        let clean_prefix = prefix.trim_end_matches('/');
        for other_route in other.routes {
            let mut handlers_with_mw = Vec::new();
            for (method, handler) in &other_route.handlers {
                let mut current_handler = handler.clone();
                for mw in other.middlewares.iter().rev() {
                    let next_handler = current_handler.clone();
                    let mw_clone = mw.clone();
                    current_handler = Arc::new(MiddlewareHandler {
                        mw: mw_clone,
                        next: next_handler,
                    });
                }
                handlers_with_mw.push((method.clone(), current_handler));
            }
            let nested_path = format!("{}{}", clean_prefix, other_route.path);
            self.routes.push(Arc::new(Route {
                path: nested_path,
                handlers: handlers_with_mw,
                name: other_route.name.clone(),
            }));
        }
        self
    }

    pub fn prefix(mut self, prefix: &str) -> Self {
        let clean_prefix = prefix.trim_end_matches('/');
        let clean_prefix = if clean_prefix.starts_with('/') {
            clean_prefix.to_string()
        } else {
            format!("/{}", clean_prefix)
        };
        for route in &mut self.routes {
            let mut path = route.path.clone();
            if !path.starts_with('/') {
                path = format!("/{}", path);
            }
            let new_path = format!("{}{}", clean_prefix, path);
            *route = Arc::new(Route {
                path: new_path,
                handlers: route.handlers.clone(),
                name: route.name.clone(),
            });
        }
        self
    }

    pub fn layer(mut self, mw: crate::middleware::MiddlewareFn) -> Self {
        self.middlewares.push(mw);
        self
    }
}
