use std::marker::PhantomData;
use serde_json::Value;
use super::error::Error;
use super::any::{AnyQueryResult, Executor};
use super::row::AnyRow;

pub struct Query<'q, DB = super::any::Any, Args = super::any::AnyArguments<'q>> {
    pub sql: &'q str,
    pub arguments: Vec<Value>,
    pub _marker: PhantomData<(DB, Args)>,
}

impl<'q, DB: super::any::Database, Args> Query<'q, DB, Args> {
    pub fn bind<T>(mut self, value: T) -> Self
    where
        T: IntoBindValue,
    {
        self.arguments.push(value.into_bind_value());
        self
    }

    pub async fn execute<E>(self, executor: E) -> Result<AnyQueryResult, Error>
    where
        E: Executor<Database = DB>,
    {
        executor.execute(self.sql, &self.arguments).await
    }

    pub async fn fetch_all<E>(self, executor: E) -> Result<Vec<AnyRow>, Error>
    where
        E: Executor<Database = DB>,
    {
        executor.fetch_all(self.sql, &self.arguments).await
    }

    pub async fn fetch_optional<E>(self, executor: E) -> Result<Option<AnyRow>, Error>
    where
        E: Executor<Database = DB>,
    {
        executor.fetch_optional(self.sql, &self.arguments).await
    }

    pub async fn fetch_one<E>(self, executor: E) -> Result<AnyRow, Error>
    where
        E: Executor<Database = DB>,
    {
        executor.fetch_one(self.sql, &self.arguments).await
    }
}

pub fn query<'q, DB>(sql: &'q str) -> Query<'q, DB, super::any::AnyArguments<'q>> {
    Query {
        sql,
        arguments: Vec::new(),
        _marker: PhantomData,
    }
}

pub trait IntoBindValue {
    fn into_bind_value(self) -> Value;
}

impl IntoBindValue for Value {
    fn into_bind_value(self) -> Value {
        self
    }
}

impl IntoBindValue for &Value {
    fn into_bind_value(self) -> Value {
        self.clone()
    }
}

impl IntoBindValue for &str {
    fn into_bind_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoBindValue for String {
    fn into_bind_value(self) -> Value {
        Value::String(self)
    }
}

impl IntoBindValue for &String {
    fn into_bind_value(self) -> Value {
        Value::String(self.clone())
    }
}

impl IntoBindValue for i64 {
    fn into_bind_value(self) -> Value {
        Value::Number(serde_json::Number::from(self))
    }
}

impl IntoBindValue for i32 {
    fn into_bind_value(self) -> Value {
        Value::Number(serde_json::Number::from(self))
    }
}

impl IntoBindValue for u64 {
    fn into_bind_value(self) -> Value {
        Value::Number(serde_json::Number::from(self))
    }
}

impl IntoBindValue for f64 {
    fn into_bind_value(self) -> Value {
        if let Some(n) = serde_json::Number::from_f64(self) {
            Value::Number(n)
        } else {
            Value::Null
        }
    }
}

impl IntoBindValue for bool {
    fn into_bind_value(self) -> Value {
        Value::Bool(self)
    }
}

impl<T: IntoBindValue> IntoBindValue for Option<T> {
    fn into_bind_value(self) -> Value {
        match self {
            Some(v) => v.into_bind_value(),
            None => Value::Null,
        }
    }
}
