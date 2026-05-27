use crate::validator::Validate;
use std::collections::HashMap;

pub struct Validator;

impl Validator {
    /// Validate a struct that implements our custom Validate trait, returning a Map of field errors
    pub fn validate<T: Validate>(data: &T) -> Result<(), HashMap<String, String>> {
        data.validate()
    }
}
