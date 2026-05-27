use std::collections::HashMap;

pub trait Validate {
    fn validate(&self) -> Result<(), HashMap<String, String>>;
}
