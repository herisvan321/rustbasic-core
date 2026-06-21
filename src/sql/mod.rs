pub mod any;
pub mod error;
pub mod query;
pub mod row;

pub use error::Error;
pub use query::{query, Query};
pub use row::{AnyRow, AnyColumn, AnyTypeInfo, Row, Column, TypeInfo, Decode, RowIndex};
pub use any::{Any, AnyPool, AnyConnection, AnyQueryResult, PoolConnection};
#[cfg(feature = "mysql")]
pub use any::MySqlPool;

#[macro_export]
macro_rules! sql_params {
    ($($val:expr),* $(,)?) => {
        vec![
            $(
                $crate::serde_json::to_value(&$val).unwrap_or($crate::serde_json::Value::Null)
            ),*
        ]
    };
}
