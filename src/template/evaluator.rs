use super::parser::{Expr, Node};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub type FilterFn = Arc<dyn Fn(&Value, &[Value]) -> Value + Send + Sync>;

pub struct Evaluator {
    filters: HashMap<String, FilterFn>,
}

impl Evaluator {
    pub fn new(filters: HashMap<String, FilterFn>) -> Self {
        Self { filters }
    }

    pub fn evaluate_expr(&self, expr: &Expr, context: &Value) -> Value {
        match expr {
            Expr::StringLiteral(s) => Value::String(s.clone()),
            Expr::NumberLiteral(n) => {
                if let Some(num) = serde_json::Number::from_f64(*n) {
                    Value::Number(num)
                } else {
                    Value::Null
                }
            }
            Expr::BooleanLiteral(b) => Value::Bool(*b),
            Expr::Path(path) => {
                let mut current = context;
                for segment in path {
                    if let Value::Object(map) = current {
                        if let Some(next) = map.get(segment) {
                            current = next;
                        } else {
                            return Value::Null;
                        }
                    } else {
                        return Value::Null;
                    }
                }
                current.clone()
            }
            Expr::Comparison { left, op, right } => {
                let left_val = self.evaluate_expr(left, context);
                let right_val = self.evaluate_expr(right, context);

                let res = match op.as_str() {
                    "==" => left_val == right_val,
                    "!=" => left_val != right_val,
                    "<" => match (&left_val, &right_val) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() < r.as_f64(),
                        (Value::String(l), Value::String(r)) => l < r,
                        _ => false,
                    },
                    ">" => match (&left_val, &right_val) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() > r.as_f64(),
                        (Value::String(l), Value::String(r)) => l > r,
                        _ => false,
                    },
                    "<=" => match (&left_val, &right_val) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() <= r.as_f64(),
                        (Value::String(l), Value::String(r)) => l <= r,
                        _ => false,
                    },
                    ">=" => match (&left_val, &right_val) {
                        (Value::Number(l), Value::Number(r)) => l.as_f64() >= r.as_f64(),
                        (Value::String(l), Value::String(r)) => l >= r,
                        _ => false,
                    },
                    _ => false,
                };
                Value::Bool(res)
            }
            Expr::Filter { expr, filter_name, args } => {
                let val = self.evaluate_expr(expr, context);
                let evaluated_args: Vec<Value> = args.iter()
                    .map(|a| self.evaluate_expr(a, context))
                    .collect();

                if let Some(filter_fn) = self.filters.get(filter_name) {
                    filter_fn(&val, &evaluated_args)
                } else {
                    val
                }
            }
        }
    }

    pub fn render_nodes(&self, nodes: &[Node], context: &Value) -> Result<String, String> {
        let mut output = String::new();
        for node in nodes {
            match node {
                Node::Text(t) => {
                    output.push_str(t);
                }
                Node::Variable(expr) => {
                    let val = self.evaluate_expr(expr, context);
                    match val {
                        Value::Null => {}
                        Value::String(s) => output.push_str(&s),
                        Value::Number(n) => output.push_str(&n.to_string()),
                        Value::Bool(b) => output.push_str(&b.to_string()),
                        other => {
                            output.push_str(&other.to_string());
                        }
                    }
                }
                Node::If { condition, then_branch, else_branch } => {
                    let cond_val = self.evaluate_expr(condition, context);
                    let is_truthy = match cond_val {
                        Value::Null => false,
                        Value::Bool(b) => b,
                        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                        Value::String(s) => !s.is_empty(),
                        Value::Array(a) => !a.is_empty(),
                        Value::Object(o) => !o.is_empty(),
                    };

                    if is_truthy {
                        let sub = self.render_nodes(then_branch, context)?;
                        output.push_str(&sub);
                    } else if let Some(else_nodes) = else_branch {
                        let sub = self.render_nodes(else_nodes, context)?;
                        output.push_str(&sub);
                    }
                }
                Node::For { item, iterator, body } => {
                    let iter_val = self.evaluate_expr(iterator, context);
                    if let Value::Array(list) = iter_val {
                        for val in list {
                            let mut local_context = context.clone();
                            if let Value::Object(ref mut map) = local_context {
                                map.insert(item.clone(), val.clone());
                            } else {
                                let mut map = serde_json::Map::new();
                                map.insert(item.clone(), val.clone());
                                local_context = Value::Object(map);
                            }
                            let sub = self.render_nodes(body, &local_context)?;
                            output.push_str(&sub);
                        }
                    }
                }
            }
        }
        Ok(output)
    }
}
