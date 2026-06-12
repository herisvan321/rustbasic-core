#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Text(String),
    VariableStart,     // {{
    VariableEnd,       // }}
    BlockStart,        // {%
    BlockEnd,          // %}
    Identifier(String),// e.g. names, keywords (if, for, in, endif, endfor)
    StringLiteral(String),
    NumberLiteral(f64),
    Operator(String),  // =, ==, !=, <, >, <=, >=, |
    Dot,               // .
    Comma,             // ,
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    in_expression: bool,
    _input: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            in_expression: false,
            _input: input,
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.chars.len() {
            Some(self.chars[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let ch = self.chars[self.pos];
            self.pos += 1;
            Some(ch)
        } else {
            None
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while self.pos < self.chars.len() {
            if !self.in_expression {
                let mut text = String::new();
                let mut found_delim = false;

                while self.pos < self.chars.len() {
                    let ch = self.chars[self.pos];
                    let next = self.peek_next();

                    if ch == '{' && next == Some('{') {
                        found_delim = true;
                        if !text.is_empty() {
                            tokens.push(Token::Text(std::mem::take(&mut text)));
                        }
                        tokens.push(Token::VariableStart);
                        self.advance(); // consume '{'
                        self.advance(); // consume '{'
                        self.in_expression = true;
                        break;
                    } else if ch == '{' && next == Some('%') {
                        found_delim = true;
                        if !text.is_empty() {
                            tokens.push(Token::Text(std::mem::take(&mut text)));
                        }
                        tokens.push(Token::BlockStart);
                        self.advance(); // consume '{'
                        self.advance(); // consume '%'
                        self.in_expression = true;
                        break;
                    } else {
                        text.push(ch);
                        self.pos += 1;
                    }
                }

                if !found_delim && !text.is_empty() {
                    tokens.push(Token::Text(text));
                }
            } else {
                self.skip_whitespace();
                if self.pos >= self.chars.len() {
                    break;
                }

                let ch = self.chars[self.pos];
                let next = self.peek_next();

                if ch == '}' && next == Some('}') {
                    tokens.push(Token::VariableEnd);
                    self.advance(); // consume '}'
                    self.advance(); // consume '}'
                    self.in_expression = false;
                } else if ch == '%' && next == Some('}') {
                    tokens.push(Token::BlockEnd);
                    self.advance(); // consume '%'
                    self.advance(); // consume '}'
                    self.in_expression = false;
                } else if ch == '\'' || ch == '"' {
                    let quote = ch;
                    self.advance(); // consume quote
                    let mut literal = String::new();
                    let mut escaped = false;
                    while self.pos < self.chars.len() {
                        let current_ch = self.chars[self.pos];
                        if escaped {
                            literal.push(current_ch);
                            escaped = false;
                            self.pos += 1;
                        } else if current_ch == '\\' {
                            escaped = true;
                            self.pos += 1;
                        } else if current_ch == quote {
                            self.pos += 1; // consume quote
                            break;
                        } else {
                            literal.push(current_ch);
                            self.pos += 1;
                        }
                    }
                    tokens.push(Token::StringLiteral(literal));
                } else if ch.is_ascii_digit() {
                    let mut num_str = String::new();
                    while self.pos < self.chars.len() {
                        let current_ch = self.chars[self.pos];
                        if current_ch.is_ascii_digit() || current_ch == '.' {
                            num_str.push(current_ch);
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    let val: f64 = num_str.parse().map_err(|e| format!("Invalid number: {}", e))?;
                    tokens.push(Token::NumberLiteral(val));
                } else if ch.is_alphabetic() || ch == '_' {
                    let mut ident = String::new();
                    while self.pos < self.chars.len() {
                        let current_ch = self.chars[self.pos];
                        if current_ch.is_alphanumeric() || current_ch == '_' {
                            ident.push(current_ch);
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                    tokens.push(Token::Identifier(ident));
                } else if ch == '.' {
                    tokens.push(Token::Dot);
                    self.pos += 1;
                } else if ch == ',' {
                    tokens.push(Token::Comma);
                    self.pos += 1;
                } else if ch == '=' && next == Some('=') {
                    tokens.push(Token::Operator("==".to_string()));
                    self.advance();
                    self.advance();
                } else if ch == '!' && next == Some('=') {
                    tokens.push(Token::Operator("!=".to_string()));
                    self.advance();
                    self.advance();
                } else if ch == '<' && next == Some('=') {
                    tokens.push(Token::Operator("<=".to_string()));
                    self.advance();
                    self.advance();
                } else if ch == '>' && next == Some('=') {
                    tokens.push(Token::Operator(">=".to_string()));
                    self.advance();
                    self.advance();
                } else if ch == '<' {
                    tokens.push(Token::Operator("<".to_string()));
                    self.pos += 1;
                } else if ch == '>' {
                    tokens.push(Token::Operator(">".to_string()));
                    self.pos += 1;
                } else if ch == '|' {
                    tokens.push(Token::Operator("|".to_string()));
                    self.pos += 1;
                } else {
                    tokens.push(Token::Operator(ch.to_string()));
                    self.pos += 1;
                }
            }
        }

        Ok(tokens)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }
}
