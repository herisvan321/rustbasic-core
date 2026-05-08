use axum::{
    extract::{FromRequest, FromRequestParts, Query, Form, Request as AxumRequest},
    http::Method,
    response::{IntoResponse, Response},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use validator::Validate;
use axum_session::Session;
use crate::session_manager::RustBasicSessionStore;

pub struct Request {
    pub inputs: Value,
    pub method: Method,
    #[allow(dead_code)]
    pub headers: HashMap<String, String>,
    pub session: Session<RustBasicSessionStore>,
}

impl Request {
    #[allow(dead_code)]
    pub fn input(&self, key: &str) -> Option<&Value> {
        self.inputs.get(key)
    }

    pub fn input_as_str(&self, key: &str) -> Option<&str> {
        self.inputs.get(key).and_then(|v| v.as_str())
    }

    pub fn query(&self, key: &str) -> Option<&str> {
        self.input_as_str(key)
    }

    pub fn all(&self) -> &Value {
        &self.inputs
    }

    pub fn validate<T: Validate + serde::de::DeserializeOwned>(&self) -> Result<T, Box<(axum::http::StatusCode, Response)>> {
        let data: T = serde_json::from_value(self.inputs.clone()).map_err(|e| {
            Box::new((axum::http::StatusCode::UNPROCESSABLE_ENTITY, 
             axum::response::Json(json!({ "error": "Invalid format", "detail": e.to_string() })).into_response()))
        })?;

        data.validate().map_err(|e| {
            // Simpan input lama ke session untuk repopulasi form (Flash Input)
            self.session.set("old", self.inputs.clone());
            
            Box::new((axum::http::StatusCode::UNPROCESSABLE_ENTITY, 
             axum::response::Json(json!({ "errors": e })).into_response()))
        })?;

        Ok(data)
    }
}

impl<S> FromRequest<S> for Request
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: AxumRequest, state: &S) -> Result<Self, Self::Rejection> {
        let method = req.method().clone();
        let mut inputs = json!({});

        // 1. Ambil Query Params (?id=1)
        let (mut parts, body) = req.into_parts();
        if let Ok(Query(query_params)) = Query::<HashMap<String, String>>::from_request_parts(&mut parts, state).await {
            for (k, v) in query_params {
                inputs[k] = json!(v);
            }
        }

        // 2. Ambil Form Data (POST)
        let parts_copy = parts.clone();
        if method == Method::POST
            && let Ok(Form(form_data)) = Form::<HashMap<String, String>>::from_request(axum::http::Request::from_parts(parts_copy, body), state).await {
                for (k, v) in form_data {
                    inputs[k] = json!(v);
                }
            }
        
        // Ambil Session dari extensions
        let session = parts.extensions
            .get::<Session<RustBasicSessionStore>>()
            .cloned()
            .ok_or_else(|| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Session tidak ditemukan").into_response())?;

        Ok(Request {
            inputs,
            method,
            headers: HashMap::new(),
            session,
        })
    }
}
