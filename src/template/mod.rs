pub mod lexer;
pub mod parser;
pub mod evaluator;

use lexer::Lexer;
use parser::Parser;
use evaluator::{Evaluator, FilterFn};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TemplateEngine {
    filters: HashMap<String, FilterFn>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut filters = HashMap::new();

        // Register default filters
        filters.insert(
            "safe".to_string(),
            Arc::new(|val: &Value, _args: &[Value]| val.clone()) as FilterFn,
        );
        filters.insert(
            "tojson".to_string(),
            Arc::new(|val: &Value, _args: &[Value]| {
                // Return raw string to be rendered directly
                let s = serde_json::to_string(val).unwrap_or_default();
                Value::String(s)
            }) as FilterFn,
        );

        Self { filters }
    }

    pub fn add_filter<F>(&mut self, name: &str, filter: F)
    where
        F: Fn(&Value, &[Value]) -> Value + Send + Sync + 'static,
    {
        self.filters.insert(name.to_string(), Arc::new(filter));
    }

    pub fn render(&self, template_str: &str, context: &Value) -> Result<String, String> {
        let mut lexer = Lexer::new(template_str);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let nodes = parser.parse()?;
        let evaluator = Evaluator::new(self.filters.clone());
        evaluator.render_nodes(&nodes, context)
    }
}
