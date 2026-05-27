use serde_json::{json, Value};
use std::collections::HashMap;
use crate::validator::Validate;
use crate::router::{Response, IntoResponse, Json};
use crate::session::Session;

#[derive(Clone)]
pub struct Request {
    pub inputs: Value,
    pub method: http::Method,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub session: Session,
    pub state: crate::AppState,
    pub ip_address: String,
    /// Route parameters, misal dari "/user/{id}" → params["id"] = "123"
    pub params: HashMap<String, String>,
}

impl Request {
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

    /// Ambil route parameter, misal `req.param("id")` dari route "/user/{id}"
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    pub fn validate<T: Validate + serde::de::DeserializeOwned>(&self) -> Result<T, Box<(http::StatusCode, Response)>> {
        let data: T = serde_json::from_value(self.inputs.clone()).map_err(|e| {
            Box::new((http::StatusCode::UNPROCESSABLE_ENTITY, 
             Json(json!({ "error": "Invalid format", "detail": e.to_string() })).into_response()))
        })?;

        data.validate().map_err(|errors| {
            // Simpan input lama ke session untuk repopulasi form (Flash Input)
            self.session.set("old", self.inputs.clone());
            
            // Simpan error di session untuk keperluan Inertia/Redirect
            self.session.set("errors", errors.clone());
            
            Box::new((http::StatusCode::UNPROCESSABLE_ENTITY, 
             Json(json!({ "errors": errors })).into_response()))
        })?;

        Ok(data)
    }
}
