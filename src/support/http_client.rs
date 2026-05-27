use std::time::Duration;
use serde::Serialize;
use serde::de::DeserializeOwned;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub struct Http;

pub struct PendingRequest {
    method: reqwest::Method,
    url: String,
    headers: HeaderMap,
    query: Option<serde_json::Value>,
    json_body: Option<serde_json::Value>,
    timeout: Option<Duration>,
}

pub struct Response {
    inner: reqwest::Response,
}

impl Http {
    pub fn get(url: &str) -> PendingRequest {
        PendingRequest::new(reqwest::Method::GET, url)
    }

    pub fn post(url: &str) -> PendingRequest {
        PendingRequest::new(reqwest::Method::POST, url)
    }

    pub fn put(url: &str) -> PendingRequest {
        PendingRequest::new(reqwest::Method::PUT, url)
    }

    pub fn patch(url: &str) -> PendingRequest {
        PendingRequest::new(reqwest::Method::PATCH, url)
    }

    pub fn delete(url: &str) -> PendingRequest {
        PendingRequest::new(reqwest::Method::DELETE, url)
    }
}

impl PendingRequest {
    pub fn new(method: reqwest::Method, url: &str) -> Self {
        Self {
            method,
            url: url.to_string(),
            headers: HeaderMap::new(),
            query: None,
            json_body: None,
            timeout: None,
        }
    }

    pub fn with_headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        for (k, v) in headers {
            if let (Ok(hname), Ok(hval)) = (HeaderName::from_bytes(k.as_bytes()), HeaderValue::from_str(&v)) {
                self.headers.insert(hname, hval);
            }
        }
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        if let (Ok(hname), Ok(hval)) = (HeaderName::from_bytes(key.as_bytes()), HeaderValue::from_str(value)) {
            self.headers.insert(hname, hval);
        }
        self
    }

    pub fn with_token(self, token: &str) -> Self {
        self.header("Authorization", &format!("Bearer {}", token))
    }

    pub fn basic_auth(self, username: &str, password: Option<&str>) -> Self {
        let auth_str = format!("{}:{}", username, password.unwrap_or(""));
        let encoded = crate::base64::encode(auth_str.as_bytes());
        self.header("Authorization", &format!("Basic {}", encoded))
    }

    pub fn query(mut self, query: impl Serialize) -> Self {
        self.query = Some(serde_json::to_value(query).unwrap_or(serde_json::Value::Null));
        self
    }

    pub fn json(mut self, body: impl Serialize) -> Self {
        self.json_body = Some(serde_json::to_value(body).unwrap_or(serde_json::Value::Null));
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub async fn send(self) -> Result<Response, reqwest::Error> {
        let mut client_builder = reqwest::Client::builder();
        if let Some(t) = self.timeout {
            client_builder = client_builder.timeout(t);
        }
        let client = client_builder.build()?;

        let mut req_builder = client.request(self.method, &self.url);
        req_builder = req_builder.headers(self.headers);

        if let Some(q) = self.query {
            req_builder = req_builder.query(&q);
        }

        if let Some(b) = self.json_body {
            req_builder = req_builder.json(&b);
        }

        let resp = req_builder.send().await?;
        Ok(Response { inner: resp })
    }
}

impl Response {
    pub fn status(&self) -> reqwest::StatusCode {
        self.inner.status()
    }

    pub fn is_success(&self) -> bool {
        self.inner.status().is_success()
    }

    pub async fn text(self) -> Result<String, reqwest::Error> {
        self.inner.text().await
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T, reqwest::Error> {
        self.inner.json::<T>().await
    }

    pub async fn json_value(self) -> Result<serde_json::Value, reqwest::Error> {
        self.inner.json::<serde_json::Value>().await
    }
}
