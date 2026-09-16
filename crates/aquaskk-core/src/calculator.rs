//! Arithmetic evaluator for the gadget dictionary's `=expr` conversion
//! (port of `calculator.h`).
//!
//! ```text
//! expression = term { ('+' | '-') term }
//! term       = primary { ('*' | '/' | '%') primary }
//! primary    = [ '+' | '-' ] number | '(' expression ')'
//! ```

#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    /// 計算エラー:不正な文字です
    InvalidCharacter,
    /// 計算エラー:ゼロ除算です
    DivideByZero,
    /// 計算エラー:')'が見つかりませんでした
    UnbalancedParen,
    /// 計算エラー:計算できませんでした
    Incomplete,
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            CalcError::InvalidCharacter => "計算エラー:不正な文字です",
            CalcError::DivideByZero => "計算エラー:ゼロ除算です",
            CalcError::UnbalancedParen => "計算エラー:')'が見つかりませんでした",
            CalcError::Incomplete => "計算エラー:計算できませんでした",
        };
        write!(f, "{message}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Op(char),
    Number(f64),
    Eof,
}

struct Parser<'a> {
    input: std::iter::Peekable<std::str::Chars<'a>>,
    buffer: Option<Token>,
}

impl Parser<'_> {
    fn get_token(&mut self) -> Result<Token, CalcError> {
        if let Some(token) = self.buffer.take() {
            return Ok(token);
        }

        // Skip whitespace like `istream >>`
        while matches!(self.input.peek(), Some(c) if c.is_whitespace()) {
            self.input.next();
        }

        match self.input.peek() {
            None => Ok(Token::Eof),
            Some(&c) => match c {
                '(' | ')' | '+' | '-' | '*' | '/' | '%' => {
                    self.input.next();
                    Ok(Token::Op(c))
                }
                '.' | '0'..='9' => {
                    let mut literal = String::new();
                    while matches!(self.input.peek(), Some(&c) if c.is_ascii_digit() || c == '.') {
                        literal.push(self.input.next().unwrap());
                    }
                    literal.parse().map(Token::Number).map_err(|_| CalcError::InvalidCharacter)
                }
                _ => Err(CalcError::InvalidCharacter),
            },
        }
    }

    fn save_token(&mut self, token: Token) {
        self.buffer = Some(token);
    }

    fn expression(&mut self) -> Result<f64, CalcError> {
        let mut left = self.term()?;

        loop {
            match self.get_token()? {
                Token::Op('+') => left += self.term()?,
                Token::Op('-') => left -= self.term()?,
                token => {
                    self.save_token(token);
                    return Ok(left);
                }
            }
        }
    }

    fn term(&mut self) -> Result<f64, CalcError> {
        let mut left = self.primary()?;

        loop {
            match self.get_token()? {
                Token::Op('*') => left *= self.primary()?,
                Token::Op('/') => {
                    let divisor = self.primary()?;
                    if divisor == 0.0 {
                        return Err(CalcError::DivideByZero);
                    }
                    left /= divisor;
                }
                Token::Op('%') => left %= self.primary()?,
                token => {
                    self.save_token(token);
                    return Ok(left);
                }
            }
        }
    }

    fn primary(&mut self) -> Result<f64, CalcError> {
        match self.get_token()? {
            Token::Op('(') => {
                let value = self.expression()?;

                match self.get_token()? {
                    Token::Op(')') => Ok(value),
                    _ => Err(CalcError::UnbalancedParen),
                }
            }
            Token::Number(value) => Ok(value),
            Token::Op('+') => self.primary(),
            Token::Op('-') => Ok(-self.primary()?),
            _ => Err(CalcError::Incomplete),
        }
    }
}

/// Evaluate an arithmetic expression.
pub fn run(expression: &str) -> Result<f64, CalcError> {
    let mut parser = Parser { input: expression.chars().peekable(), buffer: None };

    let value = parser.expression()?;

    // Trailing garbage means an invalid expression
    match parser.get_token()? {
        Token::Eof => Ok(value),
        _ => Err(CalcError::InvalidCharacter),
    }
}

/// Format a result the way `ostream <<` would (trim trailing zeros).
pub fn format(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        let formatted = format!("{value}");
        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        assert_eq!(run("100"), Ok(100.0));
        assert_eq!(run("1+2"), Ok(3.0));
        assert_eq!(run("1.2-0.2"), Ok(1.0));
        assert_eq!(run("4*.3"), Ok(1.2));
        assert_eq!(run("300/50"), Ok(6.0));
        assert_eq!(run("4%2"), Ok(0.0));
        assert_eq!(run("9.6/2"), Ok(4.8));
        assert_eq!(run("3+2*5"), Ok(13.0));
        assert_eq!(run("(3+2)*5"), Ok(25.0));
        assert_eq!(run("-3+5"), Ok(2.0));
    }

    #[test]
    fn errors() {
        assert_eq!(run("1/0"), Err(CalcError::DivideByZero));
        assert_eq!(run("("), Err(CalcError::Incomplete));
        assert_eq!(run("a"), Err(CalcError::InvalidCharacter));
        assert!(run("").is_err());
        assert_eq!(run("1+2a"), Err(CalcError::InvalidCharacter));
    }

    #[test]
    fn formatting() {
        assert_eq!(format(6.0), "6");
        assert_eq!(format(4.8), "4.8");
        assert_eq!(format(33554432.0), "33554432");
    }
}
