use crate::app::Config;
use crate::server::AppState;
use crate::router::{Router, Response};
use crate::requests::Request;
use crate::session::Session;
use crate::session_manager::RustBasicSessionStore;
use std::sync::Arc;
use crate::rand::distr::SampleString;

#[derive(Clone)]
pub struct TestClient {
    pub state: AppState,
    pub router: Router<AppState>,
    pub session_store: RustBasicSessionStore,
}

impl TestClient {
    pub async fn new(cfg: Config, router: Router<AppState>) -> Self {
        // Populate named routes
        let mut routes_map = std::collections::HashMap::new();
        for r in &router.routes {
            if let Some(ref name) = r.name {
                routes_map.insert(name.clone(), r.path.clone());
            }
        }
        let _ = crate::router::NAMED_ROUTES.set(routes_map);

        // 1. Hubungkan Database
        let db = crate::database::connect(&cfg).await;
        
        // 2. Setup Session Store
        crate::session::init_sessions(&cfg).await;
        let session_store = crate::session::setup_session(&cfg).await;
        
        Self {
            state: AppState {
                db,
                config: Arc::new(cfg),
            },
            router,
            session_store,
        }
    }

    pub async fn get(&self, path: &str) -> TestResponse {
        self.send_request("GET", path, None, None).await
    }

    pub async fn post(&self, path: &str, body: serde_json::Value) -> TestResponse {
        self.send_request("POST", path, Some(body), None).await
    }

    pub async fn put(&self, path: &str, body: serde_json::Value) -> TestResponse {
        self.send_request("PUT", path, Some(body), None).await
    }

    pub async fn patch(&self, path: &str, body: serde_json::Value) -> TestResponse {
        self.send_request("PATCH", path, Some(body), None).await
    }

    pub async fn delete(&self, path: &str) -> TestResponse {
        self.send_request("DELETE", path, None, None).await
    }

    pub async fn send_request(
        &self,
        method_str: &str,
        path: &str,
        body_json: Option<serde_json::Value>,
        headers_opt: Option<std::collections::HashMap<String, String>>,
    ) -> TestResponse {
        let method = http::Method::from_bytes(method_str.as_bytes()).unwrap();
        let inputs = body_json.unwrap_or_else(|| serde_json::json!({}));
        let mut headers = headers_opt.unwrap_or_default();

        let id = crate::rand::distr::Alphanumeric.sample_string(&mut crate::rand::rng(), 40);
        let session = Session::new(id.clone());

        // Pastikan ada token CSRF terdaftar agar lolos CSRF protection untuk POST/PUT/PATCH/DELETE
        let token = crate::rand::distr::Alphanumeric.sample_string(&mut crate::rand::rng(), 40);
        session.set("_token", token.clone());
        
        // Simulasikan token di header
        if !headers.contains_key("x-csrf-token") {
            headers.insert("x-csrf-token".to_string(), token);
        }

        let req = Request {
            inputs,
            method: method.clone(),
            path: path.to_string(),
            headers,
            session: session.clone(),
            state: self.state.clone(),
            ip_address: "127.0.0.1".to_string(),
            params: std::collections::HashMap::new(),
        };

        struct RouteDispatcher {
            router: Router<AppState>,
        }

        #[crate::async_trait]
        impl crate::router::ErasedHandler for RouteDispatcher {
            async fn call(&self, req: Request) -> Response {
                let method = req.method.clone();
                let path = req.path.clone();
                
                let mut matched_handler = None;
                let mut matched_params = std::collections::HashMap::new();
                for route in &self.router.routes {
                    if crate::server::match_path(&route.path, &path) {
                        for (m, h) in &route.handlers {
                            if m == &method {
                                matched_handler = Some(h.clone());
                                matched_params = crate::server::extract_params(&route.path, &path);
                                break;
                            }
                        }
                    }
                    if matched_handler.is_some() {
                        break;
                    }
                }
                
                if let Some(handler) = matched_handler {
                    let mut req = req;
                    req.params = matched_params;
                    let mut chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::End(handler));
                    for mw in self.router.middlewares.iter().rev() {
                        chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::Next(mw.clone(), chain));
                    }
                    chain.next(req).await
                } else {
                    crate::errors::ErrorController::not_found().await
                }
            }
        }

        let dispatcher = std::sync::Arc::new(RouteDispatcher {
            router: self.router.clone(),
        });

        let mut chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::End(dispatcher));
        chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::Next(
            crate::middleware::from_fn(crate::middleware::security_headers::security_headers_middleware),
            chain,
        ));
        chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::Next(
            crate::middleware::from_fn(crate::middleware::logging::logging_middleware),
            chain,
        ));

        let res = chain.next(req).await;
        TestResponse { response: res }
    }
}

pub struct TestResponse {
    pub response: Response,
}

impl TestResponse {
    pub fn status(&self) -> u16 {
        self.response.status().as_u16()
    }

    pub fn text(&self) -> String {
        String::from_utf8(self.response.body().clone()).unwrap_or_default()
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(self.response.body())
    }

    pub fn assert_status(&self, code: u16) {
        assert_eq!(self.status(), code, "Response status code was {}, expected {}", self.status(), code);
    }

    pub fn assert_see(&self, val: &str) {
        let txt = self.text();
        assert!(txt.contains(val), "Response did not contain '{}'. Body: {}", val, txt);
    }

    pub fn assert_dont_see(&self, val: &str) {
        let txt = self.text();
        assert!(!txt.contains(val), "Response contained '{}' when it shouldn't. Body: {}", val, txt);
    }
}
