//! 算式解析器，限制递归深度和参数数，拒绝未消费的输入。

use super::{factorial, function};

pub(super) struct Parser<'a> {
    bytes: &'a [u8],

    position: usize,

    depth: usize,
}

impl Parser<'_> {
    pub(super) fn evaluate(text: &str) -> Option<f64> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            position: 0,
            depth: 0,
        };
        let value = parser.expression()?;
        (parser.peek().is_none() && value.is_finite()).then_some(value)
    }

    fn peek(&mut self) -> Option<u8> {
        while self
            .bytes
            .get(self.position)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.position += 1;
        }
        self.bytes.get(self.position).copied()
    }

    fn expression(&mut self) -> Option<f64> {
        self.depth += 1;
        if self.depth > 32 {
            return None;
        }
        let mut value = self.term()?;
        while let Some(op @ (b'+' | b'-')) = self.peek() {
            self.position += 1;
            let rhs = self.term()?;
            value = if op == b'+' { value + rhs } else { value - rhs };
        }
        self.depth -= 1;
        Some(value)
    }

    fn term(&mut self) -> Option<f64> {
        let mut value = self.unary()?;
        while let Some(op @ (b'*' | b'/' | b'%')) = self.peek() {
            self.position += 1;
            let rhs = self.unary()?;
            value = match op {
                b'*' => value * rhs,
                b'/' => value / rhs,
                _ => value % rhs,
            };
        }
        Some(value)
    }

    fn unary(&mut self) -> Option<f64> {
        self.depth += 1;
        if self.depth > 32 {
            return None;
        }
        let value = match self.peek() {
            Some(b'-') => {
                self.position += 1;
                -self.unary()?
            }
            Some(b'+') => {
                self.position += 1;
                self.unary()?
            }
            _ => self.power()?,
        };
        self.depth -= 1;
        Some(value)
    }

    fn power(&mut self) -> Option<f64> {
        let mut value = self.atom()?;
        loop {
            match self.peek() {
                Some(b'!') => {
                    self.position += 1;
                    value = factorial(value)?;
                }
                Some(b'%')
                    if !self
                        .bytes
                        .get(self.position + 1)
                        .is_some_and(|b| b.is_ascii_digit() || *b == b'(') =>
                {
                    self.position += 1;
                    value /= 100.0;
                }
                _ => break,
            }
        }
        if self.peek() == Some(b'^') {
            self.position += 1;
            value = value.powf(self.unary()?);
        }
        Some(value)
    }

    fn atom(&mut self) -> Option<f64> {
        if self.peek() == Some(b'(') {
            self.position += 1;
            let value = self.expression()?;
            if self.peek() != Some(b')') {
                return None;
            }
            self.position += 1;
            return Some(value);
        }
        if self.peek().is_some_and(|b| b.is_ascii_alphabetic()) {
            let begin = self.position;
            while self
                .peek()
                .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                self.position += 1;
            }
            let name = std::str::from_utf8(&self.bytes[begin..self.position]).ok()?;
            if self.peek() != Some(b'(') {
                return match name {
                    "pi" => Some(std::f64::consts::PI),
                    "e" => Some(std::f64::consts::E),
                    _ => None,
                };
            }
            self.position += 1;
            let mut args = Vec::new();
            if self.peek() != Some(b')') {
                loop {
                    if args.len() >= 64 {
                        return None;
                    }
                    args.push(self.expression()?);
                    if self.peek() != Some(b',') {
                        break;
                    }
                    self.position += 1;
                }
            }
            if self.peek() != Some(b')') {
                return None;
            }
            self.position += 1;
            return function(name, &args);
        }
        let begin = self.position;
        while self.peek().is_some_and(|b| b.is_ascii_digit() || b == b'.') {
            self.position += 1;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.position += 1;
            }
        }
        std::str::from_utf8(&self.bytes[begin..self.position])
            .ok()?
            .parse()
            .ok()
    }
}
