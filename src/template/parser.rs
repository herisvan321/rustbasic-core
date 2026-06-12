use super::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Path(Vec<String>),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    Comparison {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Filter {
        expr: Box<Expr>,
        filter_name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Text(String),
    Variable(Expr),
    If {
        condition: Expr,
        then_branch: Vec<Node>,
        else_branch: Option<Vec<Node>>,
    },
    For {
        item: String,
        iterator: Expr,
        body: Vec<Node>,
    },
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            Some(self.tokens[self.pos].clone())
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = &self.tokens[self.pos];
            self.pos += 1;
            Some(t.clone())
        } else {
            None
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), String> {
        if let Some(t) = self.peek() {
            if t == token {
                self.advance();
                Ok(())
            } else {
                Err(format!("Expected token {:?}, found {:?}", token, t))
            }
        } else {
            Err(format!("Expected token {:?}, reached EOF", token))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Node>, String> {
        let mut nodes = Vec::new();
        while self.pos < self.tokens.len() {
            if let Some(token) = self.peek() {
                match token {
                    Token::Text(text) => {
                        nodes.push(Node::Text(text));
                        self.advance();
                    }
                    Token::VariableStart => {
                        self.advance(); // consume {{
                        let expr = self.parse_expr()?;
                        self.expect(Token::VariableEnd)?;
                        nodes.push(Node::Variable(expr));
                    }
                    Token::BlockStart => {
                        self.advance(); // consume {%
                        let node = self.parse_block()?;
                        nodes.push(node);
                    }
                    _ => {
                        return Err(format!("Unexpected token in template body: {:?}", token));
                    }
                }
            }
        }
        Ok(nodes)
    }

    fn parse_block(&mut self) -> Result<Node, String> {
        if let Some(Token::Identifier(keyword)) = self.peek() {
            match keyword.as_str() {
                "if" => {
                    self.advance(); // consume "if"
                    let condition = self.parse_expr()?;
                    self.expect(Token::BlockEnd)?;

                    let mut then_branch = Vec::new();
                    let mut else_branch = None;

                    while self.pos < self.tokens.len() {
                        if let Some(Token::BlockStart) = self.peek() {
                            if let Some(Token::Identifier(ident)) = self.tokens.get(self.pos + 1) {
                                if ident == "else" {
                                    self.advance(); // consume BlockStart
                                    self.advance(); // consume "else"
                                    self.expect(Token::BlockEnd)?;

                                    let mut else_nodes = Vec::new();
                                    while self.pos < self.tokens.len() {
                                        if let Some(Token::BlockStart) = self.peek() {
                                            if let Some(Token::Identifier(inner_ident)) = self.tokens.get(self.pos + 1) {
                                                if inner_ident == "endif" {
                                                    break;
                                                }
                                            }
                                        }
                                        if let Some(Token::Text(text)) = self.peek() {
                                            else_nodes.push(Node::Text(text));
                                            self.advance();
                                        } else if let Some(Token::VariableStart) = self.peek() {
                                            self.advance();
                                            let expr = self.parse_expr()?;
                                            self.expect(Token::VariableEnd)?;
                                            else_nodes.push(Node::Variable(expr));
                                        } else if let Some(Token::BlockStart) = self.peek() {
                                            self.advance();
                                            else_nodes.push(self.parse_block()?);
                                        }
                                    }
                                    else_branch = Some(else_nodes);
                                    continue;
                                } else if ident == "endif" {
                                    self.advance(); // consume BlockStart
                                    self.advance(); // consume "endif"
                                    self.expect(Token::BlockEnd)?;
                                    break;
                                }
                            }
                        }

                        if let Some(Token::Text(text)) = self.peek() {
                            then_branch.push(Node::Text(text));
                            self.advance();
                        } else if let Some(Token::VariableStart) = self.peek() {
                            self.advance();
                            let expr = self.parse_expr()?;
                            self.expect(Token::VariableEnd)?;
                            then_branch.push(Node::Variable(expr));
                        } else if let Some(Token::BlockStart) = self.peek() {
                            self.advance();
                            then_branch.push(self.parse_block()?);
                        }
                    }

                    Ok(Node::If {
                        condition,
                        then_branch,
                        else_branch,
                    })
                }
                "for" => {
                    self.advance(); // consume "for"
                    let item = if let Some(Token::Identifier(name)) = self.advance() {
                        name
                    } else {
                        return Err("Expected item identifier in for loop".to_string());
                    };

                    if let Some(Token::Identifier(keyword_in)) = self.advance() {
                        if keyword_in != "in" {
                            return Err(format!("Expected 'in', found '{}'", keyword_in));
                        }
                    } else {
                        return Err("Expected 'in' keyword in for loop".to_string());
                    }

                    let iterator = self.parse_expr()?;
                    self.expect(Token::BlockEnd)?;

                    let mut body = Vec::new();
                    while self.pos < self.tokens.len() {
                        if let Some(Token::BlockStart) = self.peek() {
                            if let Some(Token::Identifier(ident)) = self.tokens.get(self.pos + 1) {
                                if ident == "endfor" {
                                    self.advance(); // consume BlockStart
                                    self.advance(); // consume "endfor"
                                    self.expect(Token::BlockEnd)?;
                                    break;
                                }
                            }
                        }

                        if let Some(Token::Text(text)) = self.peek() {
                            body.push(Node::Text(text));
                            self.advance();
                        } else if let Some(Token::VariableStart) = self.peek() {
                            self.advance();
                            let expr = self.parse_expr()?;
                            self.expect(Token::VariableEnd)?;
                            body.push(Node::Variable(expr));
                        } else if let Some(Token::BlockStart) = self.peek() {
                            self.advance();
                            body.push(self.parse_block()?);
                        }
                    }

                    Ok(Node::For {
                        item,
                        iterator,
                        body,
                    })
                }
                other => Err(format!("Unknown block keyword: {}", other)),
            }
        } else {
            Err("Expected keyword at start of block statement".to_string())
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison_expr()?;

        while let Some(Token::Operator(op)) = self.peek() {
            if op == "|" {
                self.advance(); // consume "|"
                if let Some(Token::Identifier(filter_name)) = self.advance() {
                    let mut args = Vec::new();
                    if let Some(Token::Operator(paren)) = self.peek() {
                        if paren == "(" {
                            self.advance(); // consume "("
                            while self.pos < self.tokens.len() {
                                if let Some(Token::Operator(close_paren)) = self.peek() {
                                    if close_paren == ")" {
                                        self.advance(); // consume ")"
                                        break;
                                    }
                                }
                                args.push(self.parse_expr()?);
                                if let Some(Token::Comma) = self.peek() {
                                    self.advance();
                                }
                            }
                        }
                    }
                    left = Expr::Filter {
                        expr: Box::new(left),
                        filter_name,
                        args,
                    };
                } else {
                    return Err("Expected filter name after '|'".to_string());
                }
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr, String> {
        let left = self.parse_primary_expr()?;

        if let Some(Token::Operator(op)) = self.peek() {
            if op == "==" || op == "!=" || op == "<" || op == ">" || op == "<=" || op == ">=" {
                self.advance(); // consume operator
                let right = self.parse_primary_expr()?;
                return Ok(Expr::Comparison {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                });
            }
        }

        Ok(left)
    }

    fn parse_primary_expr(&mut self) -> Result<Expr, String> {
        if let Some(token) = self.peek() {
            match token {
                Token::StringLiteral(s) => {
                    self.advance();
                    Ok(Expr::StringLiteral(s))
                }
                Token::NumberLiteral(n) => {
                    self.advance();
                    Ok(Expr::NumberLiteral(n))
                }
                Token::Identifier(s) if s == "true" => {
                    self.advance();
                    Ok(Expr::BooleanLiteral(true))
                }
                Token::Identifier(s) if s == "false" => {
                    self.advance();
                    Ok(Expr::BooleanLiteral(false))
                }
                Token::Identifier(first_name) => {
                    self.advance();
                    let mut path = vec![first_name];
                    while let Some(Token::Dot) = self.peek() {
                        self.advance(); // consume "."
                        if let Some(Token::Identifier(field)) = self.advance() {
                            path.push(field);
                        } else {
                            return Err("Expected identifier after '.'".to_string());
                        }
                    }
                    Ok(Expr::Path(path))
                }
                other => Err(format!("Expected primary expression, found {:?}", other)),
            }
        } else {
            Err("Expected primary expression, reached EOF".to_string())
        }
    }
}
