use super::error::Error;

pub trait Row {}
pub trait Column {}
pub trait TypeInfo {}

#[derive(Debug, Clone)]
pub enum DbValue {
    Null,
    Text(String),
    Blob(Vec<u8>),
    Integer(i64),
    Real(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct AnyTypeInfo {
    pub name: String,
}

impl TypeInfo for AnyTypeInfo {}

impl AnyTypeInfo {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone)]
pub struct AnyColumn {
    pub name: String,
    pub type_info: AnyTypeInfo,
}

impl Column for AnyColumn {}

impl AnyColumn {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn type_info(&self) -> &AnyTypeInfo {
        &self.type_info
    }
}

#[derive(Debug, Clone)]
pub struct AnyRow {
    pub columns: Vec<AnyColumn>,
    pub values: Vec<DbValue>,
}

impl Row for AnyRow {}

impl AnyRow {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn column(&self, index: usize) -> &AnyColumn {
        &self.columns[index]
    }

    pub fn try_get<T, I>(&self, index: I) -> Result<T, Error>
    where
        T: for<'r> Decode<'r>,
        I: RowIndex,
    {
        let idx = index.index(self)?;
        let val = &self.values[idx];
        T::decode(val)
    }

    pub fn get<T, I>(&self, index: I) -> T
    where
        T: for<'r> Decode<'r>,
        I: RowIndex,
    {
        self.try_get(index).unwrap()
    }
}

pub trait RowIndex {
    fn index(&self, row: &AnyRow) -> Result<usize, Error>;
}

impl RowIndex for usize {
    fn index(&self, row: &AnyRow) -> Result<usize, Error> {
        if *self < row.len() {
            Ok(*self)
        } else {
            Err(Error::ColumnIndexOutOfBounds {
                len: row.len(),
                index: *self,
            })
        }
    }
}

impl RowIndex for &str {
    fn index(&self, row: &AnyRow) -> Result<usize, Error> {
        row.columns
            .iter()
            .position(|col| col.name == *self)
            .ok_or_else(|| Error::ColumnNotFound((*self).to_string()))
    }
}

impl RowIndex for String {
    fn index(&self, row: &AnyRow) -> Result<usize, Error> {
        row.columns
            .iter()
            .position(|col| col.name == *self)
            .ok_or_else(|| Error::ColumnNotFound((*self).to_string()))
    }
}

pub trait Decode<'r>: Sized {
    fn decode(value: &'r DbValue) -> Result<Self, Error>;
}

impl<'r, T: Decode<'r>> Decode<'r> for Option<T> {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Null => Ok(None),
            other => T::decode(other).map(Some),
        }
    }
}

impl<'r> Decode<'r> for String {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Text(s) => Ok(s.clone()),
            DbValue::Integer(i) => Ok(i.to_string()),
            DbValue::Real(f) => Ok(f.to_string()),
            DbValue::Bool(b) => Ok(b.to_string()),
            DbValue::Blob(b) => String::from_utf8(b.clone())
                .map_err(|e| Error::DecodeError(e.to_string())),
            DbValue::Null => Err(Error::DecodeError("Cannot decode NULL to String".into())),
        }
    }
}

impl<'r> Decode<'r> for i64 {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Integer(i) => Ok(*i),
            DbValue::Text(s) => s.parse::<i64>().map_err(|e| Error::DecodeError(e.to_string())),
            DbValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
            DbValue::Real(f) => Ok(*f as i64),
            _ => Err(Error::DecodeError("Cannot decode to i64".into())),
        }
    }
}

impl<'r> Decode<'r> for i32 {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Integer(i) => Ok(*i as i32),
            DbValue::Text(s) => s.parse::<i32>().map_err(|e| Error::DecodeError(e.to_string())),
            DbValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
            DbValue::Real(f) => Ok(*f as i32),
            _ => Err(Error::DecodeError("Cannot decode to i32".into())),
        }
    }
}

impl<'r> Decode<'r> for u64 {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Integer(i) => Ok(*i as u64),
            DbValue::Text(s) => s.parse::<u64>().map_err(|e| Error::DecodeError(e.to_string())),
            DbValue::Bool(b) => Ok(if *b { 1 } else { 0 }),
            DbValue::Real(f) => Ok(*f as u64),
            _ => Err(Error::DecodeError("Cannot decode to u64".into())),
        }
    }
}

impl<'r> Decode<'r> for f64 {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Real(f) => Ok(*f),
            DbValue::Integer(i) => Ok(*i as f64),
            DbValue::Text(s) => s.parse::<f64>().map_err(|e| Error::DecodeError(e.to_string())),
            _ => Err(Error::DecodeError("Cannot decode to f64".into())),
        }
    }
}

impl<'r> Decode<'r> for bool {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Bool(b) => Ok(*b),
            DbValue::Integer(i) => Ok(*i != 0),
            DbValue::Text(s) => {
                let s_lower = s.to_lowercase();
                if s_lower == "true" || s_lower == "1" || s_lower == "t" || s_lower == "y" || s_lower == "yes" {
                    Ok(true)
                } else if s_lower == "false" || s_lower == "0" || s_lower == "f" || s_lower == "n" || s_lower == "no" || s_lower.is_empty() {
                    Ok(false)
                } else {
                    Err(Error::DecodeError(format!("Cannot decode '{}' to bool", s)))
                }
            }
            _ => Err(Error::DecodeError("Cannot decode to bool".into())),
        }
    }
}

impl<'r> Decode<'r> for Vec<u8> {
    fn decode(value: &'r DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Blob(b) => Ok(b.clone()),
            DbValue::Text(s) => Ok(s.as_bytes().to_vec()),
            _ => Err(Error::DecodeError("Cannot decode to Vec<u8>".into())),
        }
    }
}
