use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

type CompileResult<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Num(f32),
    Str(String),
    Ident(String),
    Fn,
    Return,
    If,
    Else,
    End,
    Do,
    While,
    For,
    In,
    Let,
    Break,
    Continue,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Dot,
    DotDot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Bang,
    And,
    Or,
    Arrow,
    Eof,
}

fn token_name(t: &Token) -> &'static str {
    match t {
        Token::Num(_) => "number",
        Token::Str(_) => "string",
        Token::Ident(_) => "identifier",
        Token::Fn => "fn",
        Token::Return => "return",
        Token::If => "if",
        Token::Else => "else",
        Token::End => "end",
        Token::Do => "do",
        Token::While => "while",
        Token::For => "for",
        Token::In => "in",
        Token::Let => "let",
        Token::Break => "break",
        Token::Continue => "continue",
        Token::LParen => "(",
        Token::RParen => ")",
        Token::LBrace => "{",
        Token::RBrace => "}",
        Token::Comma => ",",
        Token::Semicolon => ";",
        Token::Colon => ":",
        Token::Dot => ".",
        Token::DotDot => "..",
        Token::Plus => "+",
        Token::Minus => "-",
        Token::Star => "*",
        Token::Slash => "/",
        Token::Percent => "%",
        Token::Eq => "=",
        Token::EqEq => "==",
        Token::BangEq => "!=",
        Token::Lt => "<",
        Token::Gt => ">",
        Token::LtEq => "<=",
        Token::GtEq => ">=",
        Token::Bang => "!",
        Token::And => "&&",
        Token::Or => "||",
        Token::Arrow => "->",
        Token::Eof => "end-of-file",
    }
}

pub fn lex(src: &str) -> Vec<Token> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => {
                i += 1;
            }
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                let s = src[start..i].to_string();
                if i < bytes.len() && bytes[i] == b'"' {
                    i += 1;
                }
                out.push(Token::Str(s));
            }
            b'(' => {
                out.push(Token::LParen);
                i += 1;
            }
            b')' => {
                out.push(Token::RParen);
                i += 1;
            }
            b'{' => {
                out.push(Token::LBrace);
                i += 1;
            }
            b'}' => {
                out.push(Token::RBrace);
                i += 1;
            }
            b',' => {
                out.push(Token::Comma);
                i += 1;
            }
            b';' => {
                out.push(Token::Semicolon);
                i += 1;
            }
            b':' => {
                out.push(Token::Colon);
                i += 1;
            }
            b'%' => {
                out.push(Token::Percent);
                i += 1;
            }
            b'+' => {
                out.push(Token::Plus);
                i += 1;
            }
            b'*' => {
                out.push(Token::Star);
                i += 1;
            }
            b'/' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else {
                    out.push(Token::Slash);
                    i += 1;
                }
            }
            b'-' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                    out.push(Token::Arrow);
                    i += 2;
                } else {
                    out.push(Token::Minus);
                    i += 1;
                }
            }
            b'.' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    out.push(Token::DotDot);
                    i += 2;
                } else {
                    out.push(Token::Dot);
                    i += 1;
                }
            }
            b'=' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token::EqEq);
                    i += 2;
                } else {
                    out.push(Token::Eq);
                    i += 1;
                }
            }
            b'!' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token::BangEq);
                    i += 2;
                } else {
                    out.push(Token::Bang);
                    i += 1;
                }
            }
            b'<' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token::LtEq);
                    i += 2;
                } else {
                    out.push(Token::Lt);
                    i += 1;
                }
            }
            b'>' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    out.push(Token::GtEq);
                    i += 2;
                } else {
                    out.push(Token::Gt);
                    i += 1;
                }
            }
            b'&' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'&' {
                    i += 1;
                }
                out.push(Token::And);
                i += 1;
            }
            b'|' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'|' {
                    i += 1;
                }
                out.push(Token::Or);
                i += 1;
            }
            b'\'' => {
                if i + 2 < bytes.len() && bytes[i + 1] != b'\\' && bytes[i + 2] == b'\'' {
                    out.push(Token::Num(bytes[i + 1] as f32));
                    i += 3;
                } else if i + 3 < bytes.len() && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'' {
                    let ch = match bytes[i + 2] {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'0' => '\0',
                        b'\\' => '\\',
                        b'\'' => '\'',
                        c => c as char,
                    };
                    out.push(Token::Num(ch as u32 as f32));
                    i += 4;
                } else {
                    i += 1;
                }
            }
            b'0'..=b'9' => {
                let start = i;
                let v: f32 = if i + 1 < bytes.len()
                    && bytes[i] == b'0'
                    && (bytes[i + 1] == b'x' || bytes[i + 1] == b'X')
                {
                    i += 2;
                    let hex_start = i;
                    while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                        i += 1;
                    }
                    if i == hex_start {
                        0.0
                    } else {
                        let hex = &src[hex_start..i];
                        u32::from_str_radix(hex, 16).unwrap_or(0) as f32
                    }
                } else {
                    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                        i += 1;
                    }
                    let s = &src[start..i];
                    s.parse().unwrap_or(0.0)
                };
                out.push(Token::Num(v));
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let s = &src[start..i];
                out.push(match s {
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "end" => Token::End,
                    "do" => Token::Do,
                    "while" => Token::While,
                    "for" => Token::For,
                    "in" => Token::In,
                    "let" => Token::Let,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    _ => Token::Ident(s.to_string()),
                });
            }
            _ => {
                i += 1;
            }
        }
    }
    out.push(Token::Eof);
    out
}

#[derive(Debug, Clone)]
pub enum Expr {
    Num(f32),
    Var(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    Call(String, Vec<Expr>),
    Field(Box<Expr>, String),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Expr),
    Assign(String, Expr),
    ReturnVals(Vec<Expr>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Expr, Vec<Stmt>),
    Break,
    Continue,
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReturnType {
    Color,
    Char,
    CharFg,
    FgBg,
    CharFgBg,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub ret_type: ReturnType,
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.toks[self.pos]
    }

    fn next(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        self.pos += 1;
        t
    }

    fn eat(&mut self, t: &Token) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(t) {
            self.next();
            true
        } else {
            false
        }
    }

    fn err<T>(&self, msg: &str) -> CompileResult<T> {
        Err(format!(
            "parse error near token {} ({}): {}",
            self.pos,
            token_name(self.peek()),
            msg
        ))
    }

    fn expect(&mut self, t: &Token, ctx: &str) -> CompileResult<()> {
        if self.eat(t) {
            Ok(())
        } else {
            self.err(&format!("expected {} {}", token_name(t), ctx))
        }
    }

    fn expect_ident(&mut self) -> CompileResult<String> {
        match self.next() {
            Token::Ident(s) => Ok(s),
            other => Err(format!("expected identifier, got {}", token_name(&other))),
        }
    }

    fn parse_ret_type(&mut self) -> CompileResult<ReturnType> {
        if self.eat(&Token::LParen) {
            let mut parts = Vec::new();
            while !matches!(self.peek(), Token::RParen | Token::Eof) {
                parts.push(self.expect_ident()?.to_lowercase());
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen, "to close return tuple")?;
            return match parts.as_slice() {
                [a, b] if a == "char" && b == "color" => Ok(ReturnType::CharFg),
                [a, b] if a == "color" && b == "color" => Ok(ReturnType::FgBg),
                [a, b, c] if a == "char" && b == "color" && c == "color" => {
                    Ok(ReturnType::CharFgBg)
                }
                _ => Err(
                    "unsupported tuple return type; use (char,color), (color,color), or (char,color,color)"
                        .to_string(),
                ),
            };
        }

        match self.expect_ident()?.to_lowercase().as_str() {
            "char" => Ok(ReturnType::Char),
            "color" => Ok(ReturnType::Color),
            _ => Err("unsupported return type; use color or char".to_string()),
        }
    }

    fn parse_fn(&mut self) -> CompileResult<FnDef> {
        self.expect(&Token::Fn, "before function name")?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen, "after function name")?;

        let mut params = Vec::new();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            params.push(self.expect_ident()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen, "after parameter list")?;

        let ret_type = if self.eat(&Token::Arrow) {
            self.parse_ret_type()?
        } else {
            ReturnType::Color
        };

        let body = self.parse_block_end()?;
        Ok(FnDef {
            name,
            params,
            body,
            ret_type,
        })
    }

    fn parse_block_end(&mut self) -> CompileResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !matches!(self.peek(), Token::End | Token::Else | Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::End, "to close block")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> CompileResult<Stmt> {
        let stmt = match self.peek().clone() {
            Token::Let => {
                self.next();
                let name = self.expect_ident()?;
                self.expect(&Token::Eq, "after let variable")?;
                let e = self.parse_expr();
                self.eat(&Token::Semicolon);
                Stmt::Let(name, e)
            }
            Token::Return => {
                self.next();
                if self.eat(&Token::LParen) {
                    let mut vals = Vec::new();
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        vals.push(self.parse_expr());
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen, "after return tuple")?;
                    self.eat(&Token::Semicolon);
                    if vals.is_empty() {
                        return self.err("return tuple cannot be empty");
                    }
                    Stmt::ReturnVals(vals)
                } else {
                    let e = self.parse_expr();
                    self.eat(&Token::Semicolon);
                    Stmt::ReturnVals(vec![e])
                }
            }
            Token::If => {
                self.next();
                let cond = self.parse_expr();
                self.expect(&Token::Do, "after if condition")?;
                let mut then_b = Vec::new();
                while !matches!(self.peek(), Token::Else | Token::End | Token::Eof) {
                    then_b.push(self.parse_stmt()?);
                }
                let else_b = if self.eat(&Token::Else) {
                    let mut eb = Vec::new();
                    while !matches!(self.peek(), Token::End | Token::Eof) {
                        eb.push(self.parse_stmt()?);
                    }
                    eb
                } else {
                    Vec::new()
                };
                self.expect(&Token::End, "to close if")?;
                Stmt::If(cond, then_b, else_b)
            }
            Token::While => {
                self.next();
                let cond = self.parse_expr();
                self.expect(&Token::Do, "after while condition")?;
                let body = self.parse_block_end()?;
                Stmt::While(cond, body)
            }
            Token::For => {
                self.next();
                let var = self.expect_ident()?;
                self.expect(&Token::In, "in for loop")?;
                let lo = self.parse_expr();
                self.expect(&Token::DotDot, "in for range")?;
                let hi = self.parse_expr();
                self.expect(&Token::Do, "after for range")?;
                let body = self.parse_block_end()?;
                Stmt::For(var, lo, hi, body)
            }
            Token::Break => {
                self.next();
                self.eat(&Token::Semicolon);
                Stmt::Break
            }
            Token::Continue => {
                self.next();
                self.eat(&Token::Semicolon);
                Stmt::Continue
            }
            Token::Ident(name) => {
                self.next();
                if self.eat(&Token::Eq) {
                    let e = self.parse_expr();
                    self.eat(&Token::Semicolon);
                    Stmt::Assign(name, e)
                } else {
                    self.pos -= 1;
                    let e = self.parse_expr();
                    self.eat(&Token::Semicolon);
                    Stmt::Expr(e)
                }
            }
            _ => {
                let e = self.parse_expr();
                self.eat(&Token::Semicolon);
                Stmt::Expr(e)
            }
        };
        Ok(stmt)
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Expr {
        let mut l = self.parse_and();
        while matches!(self.peek(), Token::Or) {
            self.next();
            let r = self.parse_and();
            l = Expr::BinOp(Box::new(l), BinOp::Or, Box::new(r));
        }
        l
    }

    fn parse_and(&mut self) -> Expr {
        let mut l = self.parse_cmp();
        while matches!(self.peek(), Token::And) {
            self.next();
            let r = self.parse_cmp();
            l = Expr::BinOp(Box::new(l), BinOp::And, Box::new(r));
        }
        l
    }

    fn parse_cmp(&mut self) -> Expr {
        let l = self.parse_add();
        match self.peek().clone() {
            Token::EqEq => {
                self.next();
                Expr::BinOp(Box::new(l), BinOp::Eq, Box::new(self.parse_add()))
            }
            Token::BangEq => {
                self.next();
                Expr::BinOp(Box::new(l), BinOp::Ne, Box::new(self.parse_add()))
            }
            Token::Lt => {
                self.next();
                Expr::BinOp(Box::new(l), BinOp::Lt, Box::new(self.parse_add()))
            }
            Token::Gt => {
                self.next();
                Expr::BinOp(Box::new(l), BinOp::Gt, Box::new(self.parse_add()))
            }
            Token::LtEq => {
                self.next();
                Expr::BinOp(Box::new(l), BinOp::Le, Box::new(self.parse_add()))
            }
            Token::GtEq => {
                self.next();
                Expr::BinOp(Box::new(l), BinOp::Ge, Box::new(self.parse_add()))
            }
            _ => l,
        }
    }

    fn parse_add(&mut self) -> Expr {
        let mut l = self.parse_mul();
        loop {
            match self.peek().clone() {
                Token::Plus => {
                    self.next();
                    let r = self.parse_mul();
                    l = Expr::BinOp(Box::new(l), BinOp::Add, Box::new(r));
                }
                Token::Minus => {
                    self.next();
                    let r = self.parse_mul();
                    l = Expr::BinOp(Box::new(l), BinOp::Sub, Box::new(r));
                }
                _ => break,
            }
        }
        l
    }

    fn parse_mul(&mut self) -> Expr {
        let mut l = self.parse_unary();
        loop {
            match self.peek().clone() {
                Token::Star => {
                    self.next();
                    let r = self.parse_unary();
                    l = Expr::BinOp(Box::new(l), BinOp::Mul, Box::new(r));
                }
                Token::Slash => {
                    self.next();
                    let r = self.parse_unary();
                    l = Expr::BinOp(Box::new(l), BinOp::Div, Box::new(r));
                }
                Token::Percent => {
                    self.next();
                    let r = self.parse_unary();
                    l = Expr::BinOp(Box::new(l), BinOp::Mod, Box::new(r));
                }
                _ => break,
            }
        }
        l
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek().clone() {
            Token::Minus => {
                self.next();
                Expr::UnOp(UnOp::Neg, Box::new(self.parse_unary()))
            }
            Token::Bang => {
                self.next();
                Expr::UnOp(UnOp::Not, Box::new(self.parse_unary()))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_primary();
        loop {
            if self.eat(&Token::Dot) {
                let field = self.expect_ident().unwrap_or_default();
                e = Expr::Field(Box::new(e), field);
            } else {
                break;
            }
        }
        e
    }

    fn parse_primary(&mut self) -> Expr {
        match self.peek().clone() {
            Token::Num(n) => {
                self.next();
                Expr::Num(n)
            }
            Token::LParen => {
                self.next();
                let e = self.parse_expr();
                self.eat(&Token::RParen);
                e
            }
            Token::Ident(name) => {
                self.next();
                if self.eat(&Token::LParen) {
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        args.push(self.parse_expr());
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.eat(&Token::RParen);
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            _ => {
                self.next();
                Expr::Num(0.0)
            }
        }
    }
}

/// Parse source code into an AST of function definitions.
pub fn parse(src: &str) -> CompileResult<Vec<FnDef>> {
    let toks = lex(src);
    let mut p = Parser { toks, pos: 0 };
    let mut fns = Vec::new();
    while !matches!(p.peek(), Token::Eof) {
        fns.push(p.parse_fn()?);
    }
    Ok(fns)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    // stack
    PushF = 0,
    PushC = 1,
    Pop = 2,
    // locals
    Load = 3,
    Store = 4,
    // arith
    AddF = 10,
    SubF = 11,
    MulF = 12,
    DivF = 13,
    ModF = 14,
    NegF = 15,
    AbsF = 16,
    // color arith
    AddC = 20,
    SubC = 21,
    MulC = 22,
    DivC = 23,
    MulCF = 24,
    // cmp
    EqF = 30,
    NeF = 31,
    LtF = 32,
    GtF = 33,
    LeF = 34,
    GeF = 35,
    AndF = 36,
    OrF = 37,
    NotF = 38,
    // math
    Sin = 40,
    Cos = 41,
    Tan = 42,
    Asin = 43,
    Acos = 44,
    Atan = 45,
    Atan2 = 46,
    Sqrt = 47,
    Pow = 48,
    Exp = 49,
    Log = 50,
    Log2 = 51,
    Floor = 52,
    Ceil = 53,
    Round = 54,
    Fract = 55,
    Min2 = 56,
    Max2 = 57,
    Clamp = 58,
    Mix = 59,
    Step = 60,
    Smoothstep = 61,
    Sign = 62,
    Length2 = 63,
    // color
    Rgb = 70,
    Rgba = 71,
    Hsl = 72,
    Hsv = 73,
    Gray = 74,
    Mix2C = 75,
    // field
    GetR = 80,
    GetG = 81,
    GetB = 82,
    // char classify
    IsSpace = 83,
    IsDigit = 84,
    IsAlpha = 85,
    IsUpper = 86,
    IsLower = 87,
    // control
    Jmp = 90,
    JmpZ = 91,
    Ret = 99,
    // calls
    Call = 100,
    // random
    Rand = 101,
    RandBetween = 102,
    MakeGlyphCharFg = 103,
    MakeGlyphFgBg = 104,
    MakeGlyphCharFgBg = 105,
    HashF = 106,
}

#[derive(Debug, Clone)]
pub struct CompiledFn {
    pub name: String,
    pub params: u8,
    pub code: Vec<u8>,
    pub const_floats: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct CompiledShader {
    pub fns: Vec<CompiledFn>,
    pub entry: usize,
    pub entry_ret: ReturnType,
}

const MAGIC: &[u8; 4] = b"CTSL";
const VERSION: u8 = 4;

impl CompiledShader {
    /// Serialise compiled shader to `.ctsl` bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.push(self.fns.len() as u8);
        out.push(self.entry as u8);
        out.push(match self.entry_ret {
            ReturnType::Color => 0,
            ReturnType::Char => 1,
            ReturnType::CharFg => 2,
            ReturnType::FgBg => 3,
            ReturnType::CharFgBg => 4,
        });
        for f in &self.fns {
            let name = f.name.as_bytes();
            out.push(name.len() as u8);
            out.extend_from_slice(name);
            out.push(f.params);
            out.extend_from_slice(&(f.code.len() as u32).to_le_bytes());
            out.extend_from_slice(&f.code);
        }
        out
    }

    /// Deserialise `.ctsl` bytes back to a `CompiledShader`.
    pub fn from_bytes(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 8 || &data[..4] != MAGIC {
            return Err("bad magic");
        }
        let ver = data[4];
        if ver != 1 && ver != 2 && ver != 3 && ver != VERSION {
            return Err("version mismatch");
        }
        let fn_count = data[5] as usize;
        let entry = data[6] as usize;
        let (entry_ret, mut pos) = if ver >= 2 {
            let r = match data.get(7).copied() {
                Some(1) => ReturnType::Char,
                Some(2) => ReturnType::CharFg,
                Some(3) => ReturnType::FgBg,
                Some(4) => ReturnType::CharFgBg,
                _ => ReturnType::Color,
            };
            (r, 8usize)
        } else {
            (ReturnType::Color, 7usize)
        };

        let mut fns = Vec::with_capacity(fn_count);
        for _ in 0..fn_count {
            let Some(&nlen_b) = data.get(pos) else {
                return Err("truncated");
            };
            let nlen = nlen_b as usize;
            pos += 1;
            if pos + nlen > data.len() {
                return Err("truncated");
            }
            let name = std::str::from_utf8(&data[pos..pos + nlen])
                .map_err(|_| "utf8")?
                .to_string();
            pos += nlen;

            let Some(&params) = data.get(pos) else {
                return Err("truncated");
            };
            pos += 1;
            if pos + 4 > data.len() {
                return Err("truncated");
            }
            let clen = u32::from_le_bytes(data[pos..pos + 4].try_into().map_err(|_| "truncated")?)
                as usize;
            pos += 4;
            if pos + clen > data.len() {
                return Err("truncated");
            }
            let code = data[pos..pos + clen].to_vec();
            pos += clen;
            fns.push(CompiledFn {
                name,
                params,
                code,
                const_floats: Vec::new(),
            });
        }

        Ok(CompiledShader {
            fns,
            entry,
            entry_ret,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Scalar,
    Color,
    Unknown,
}

#[derive(Default)]
struct LoopCtx {
    break_patches: Vec<usize>,
    continue_patches: Vec<usize>,
}

struct Emitter<'a> {
    code: Vec<u8>,
    consts: Vec<f32>,
    locals: HashMap<String, u8>,
    local_types: HashMap<String, ValueType>,
    local_count: u8,
    fn_names: &'a [String],
    ret_type: ReturnType,
    loops: Vec<LoopCtx>,
}

impl<'a> Emitter<'a> {
    fn push_op(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    fn push_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    fn push_i16(&mut self, v: i16) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn push_f32(&mut self, v: f32) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    fn local(&mut self, name: &str) -> u8 {
        if let Some(&s) = self.locals.get(name) {
            return s;
        }
        let s = self.local_count;
        self.locals.insert(name.to_string(), s);
        self.local_count = self.local_count.saturating_add(1);
        s
    }

    fn push_bool_from_expr(&mut self, e: &Expr) -> CompileResult<()> {
        self.emit_expr(e)?;
        Ok(())
    }

    fn infer_expr_type(&self, e: &Expr) -> CompileResult<ValueType> {
        match e {
            Expr::Num(_) => Ok(ValueType::Scalar),
            Expr::Var(name) => Ok(*self.local_types.get(name).unwrap_or(&ValueType::Unknown)),
            Expr::Field(inner, field) => {
                let t = self.infer_expr_type(inner)?;
                if t != ValueType::Color && t != ValueType::Unknown {
                    return Err(format!("field access .{} requires a color value", field));
                }
                match field.as_str() {
                    "r" | "g" | "b" => Ok(ValueType::Scalar),
                    _ => Err(format!("unknown field {}", field)),
                }
            }
            Expr::UnOp(_, a) => {
                let t = self.infer_expr_type(a)?;
                if t == ValueType::Color {
                    Err("unary operator cannot be applied to color".to_string())
                } else {
                    Ok(ValueType::Scalar)
                }
            }
            Expr::BinOp(l, op, r) => {
                let lt = self.infer_expr_type(l)?;
                let rt = self.infer_expr_type(r)?;
                match op {
                    BinOp::Add
                    | BinOp::Sub
                    | BinOp::Mul
                    | BinOp::Div
                    | BinOp::Mod
                    | BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Gt
                    | BinOp::Le
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or => {
                        if lt == ValueType::Color || rt == ValueType::Color {
                            return Err(
                                "binary scalar operator cannot be applied to color".to_string()
                            );
                        }
                        Ok(ValueType::Scalar)
                    }
                }
            }
            Expr::Call(name, args) => {
                let n = name.as_str();
                let scalar_ops = [
                    "abs",
                    "sin",
                    "cos",
                    "tan",
                    "asin",
                    "acos",
                    "atan",
                    "atan2",
                    "sqrt",
                    "pow",
                    "exp",
                    "log",
                    "log2",
                    "floor",
                    "ceil",
                    "round",
                    "fract",
                    "min",
                    "max",
                    "clamp",
                    "step",
                    "smoothstep",
                    "sign",
                    "length",
                    "is_space",
                    "is_digit",
                    "is_alpha",
                    "is_upper",
                    "is_lower",
                    "time",
                    "char_code",
                    "t",
                    "i",
                    "len",
                    "x",
                    "y",
                    "col_i",
                    "row_i",
                    "seed",
                ];
                let color_ops = ["rgb", "rgba", "hsl", "hsv", "gray", "mixc", "original"];

                if scalar_ops.contains(&n) || n == "rand" {
                    return Ok(ValueType::Scalar);
                }
                if color_ops.contains(&n) {
                    return Ok(ValueType::Color);
                }
                if n == "mix" {
                    if args.len() >= 2 {
                        let ta = self.infer_expr_type(&args[0])?;
                        let tb = self.infer_expr_type(&args[1])?;
                        if ta == ValueType::Color && tb == ValueType::Color {
                            return Ok(ValueType::Color);
                        }
                    }
                    return Ok(ValueType::Scalar);
                }
                Ok(ValueType::Unknown)
            }
        }
    }

    fn emit_short_circuit(&mut self, op: BinOp, l: &Expr, r: &Expr) -> CompileResult<()> {
        match op {
            BinOp::And => {
                self.push_bool_from_expr(l)?;
                self.push_op(Op::JmpZ);
                let jmp_false_left = self.code.len();
                self.push_i16(0);

                self.push_bool_from_expr(r)?;
                self.push_op(Op::JmpZ);
                let jmp_false_right = self.code.len();
                self.push_i16(0);

                self.push_op(Op::PushF);
                self.push_f32(1.0);
                self.push_op(Op::Jmp);
                let jmp_end = self.code.len();
                self.push_i16(0);

                let false_pos = self.code.len() as i16;
                self.push_op(Op::PushF);
                self.push_f32(0.0);

                let end_pos = self.code.len() as i16;

                let off_left = false_pos - (jmp_false_left as i16 + 2);
                let bytes_left = off_left.to_le_bytes();
                self.code[jmp_false_left] = bytes_left[0];
                self.code[jmp_false_left + 1] = bytes_left[1];

                let off_right = false_pos - (jmp_false_right as i16 + 2);
                let bytes_right = off_right.to_le_bytes();
                self.code[jmp_false_right] = bytes_right[0];
                self.code[jmp_false_right + 1] = bytes_right[1];

                let off_end = end_pos - (jmp_end as i16 + 2);
                let bytes_end = off_end.to_le_bytes();
                self.code[jmp_end] = bytes_end[0];
                self.code[jmp_end + 1] = bytes_end[1];
            }
            BinOp::Or => {
                self.push_bool_from_expr(l)?;
                self.push_op(Op::JmpZ);
                let jmp_eval_r = self.code.len();
                self.push_i16(0);

                self.push_op(Op::PushF);
                self.push_f32(1.0);
                self.push_op(Op::Jmp);
                let jmp_end_from_left = self.code.len();
                self.push_i16(0);

                let eval_r_pos = self.code.len() as i16;
                self.push_bool_from_expr(r)?;
                self.push_op(Op::JmpZ);
                let jmp_false = self.code.len();
                self.push_i16(0);

                self.push_op(Op::PushF);
                self.push_f32(1.0);
                self.push_op(Op::Jmp);
                let jmp_end_from_right = self.code.len();
                self.push_i16(0);

                let false_pos = self.code.len() as i16;
                self.push_op(Op::PushF);
                self.push_f32(0.0);

                let end_pos = self.code.len() as i16;

                let off_eval_r = eval_r_pos - (jmp_eval_r as i16 + 2);
                let b_eval_r = off_eval_r.to_le_bytes();
                self.code[jmp_eval_r] = b_eval_r[0];
                self.code[jmp_eval_r + 1] = b_eval_r[1];

                let off_false = false_pos - (jmp_false as i16 + 2);
                let b_false = off_false.to_le_bytes();
                self.code[jmp_false] = b_false[0];
                self.code[jmp_false + 1] = b_false[1];

                let off_end_left = end_pos - (jmp_end_from_left as i16 + 2);
                let b_end_left = off_end_left.to_le_bytes();
                self.code[jmp_end_from_left] = b_end_left[0];
                self.code[jmp_end_from_left + 1] = b_end_left[1];

                let off_end_right = end_pos - (jmp_end_from_right as i16 + 2);
                let b_end_right = off_end_right.to_le_bytes();
                self.code[jmp_end_from_right] = b_end_right[0];
                self.code[jmp_end_from_right + 1] = b_end_right[1];
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    fn emit_expr(&mut self, e: &Expr) -> CompileResult<()> {
        match e {
            Expr::Num(n) => {
                self.push_op(Op::PushF);
                self.push_f32(*n);
            }
            Expr::Var(name) => {
                let s = *self
                    .locals
                    .get(name.as_str())
                    .ok_or_else(|| format!("undef var {name}"))?;
                self.push_op(Op::Load);
                self.push_u8(s);
            }
            Expr::Field(inner, field) => {
                self.emit_expr(inner)?;
                match field.as_str() {
                    "r" => self.push_op(Op::GetR),
                    "g" => self.push_op(Op::GetG),
                    "b" => self.push_op(Op::GetB),
                    _ => return Err(format!("unknown field {field}")),
                }
            }
            Expr::UnOp(op, a) => {
                self.emit_expr(a)?;
                match op {
                    UnOp::Neg => self.push_op(Op::NegF),
                    UnOp::Not => self.push_op(Op::NotF),
                }
            }
            Expr::BinOp(l, op, r) => {
                if matches!(op, BinOp::And | BinOp::Or) {
                    return self.emit_short_circuit(*op, l, r);
                }
                self.emit_expr(l)?;
                self.emit_expr(r)?;
                match op {
                    BinOp::Add => self.push_op(Op::AddF),
                    BinOp::Sub => self.push_op(Op::SubF),
                    BinOp::Mul => self.push_op(Op::MulF),
                    BinOp::Div => self.push_op(Op::DivF),
                    BinOp::Mod => self.push_op(Op::ModF),
                    BinOp::Eq => self.push_op(Op::EqF),
                    BinOp::Ne => self.push_op(Op::NeF),
                    BinOp::Lt => self.push_op(Op::LtF),
                    BinOp::Gt => self.push_op(Op::GtF),
                    BinOp::Le => self.push_op(Op::LeF),
                    BinOp::Ge => self.push_op(Op::GeF),
                    BinOp::And => self.push_op(Op::AndF),
                    BinOp::Or => self.push_op(Op::OrF),
                }
            }
            Expr::Call(name, args) => {
                let load_hidden = |this: &mut Self, slot: u8| {
                    this.push_op(Op::Load);
                    this.push_u8(slot);
                };

                match name.as_str() {
                    "t" => {
                        load_hidden(self, 0);
                        return Ok(());
                    }
                    "i" => {
                        load_hidden(self, 1);
                        return Ok(());
                    }
                    "len" => {
                        load_hidden(self, 2);
                        return Ok(());
                    }
                    "x" => {
                        load_hidden(self, 3);
                        return Ok(());
                    }
                    "y" => {
                        load_hidden(self, 4);
                        return Ok(());
                    }
                    "col_i" => {
                        load_hidden(self, 5);
                        return Ok(());
                    }
                    "row_i" => {
                        load_hidden(self, 6);
                        return Ok(());
                    }
                    "char_code" => {
                        load_hidden(self, 7);
                        return Ok(());
                    }
                    "original" => {
                        load_hidden(self, 8);
                        return Ok(());
                    }
                    "time" => {
                        load_hidden(self, 9);
                        return Ok(());
                    }
                    "seed" => {
                        load_hidden(self, 10);
                        return Ok(());
                    }
                    _ => {}
                }

                if name == "rand" {
                    match args.len() {
                        0 => self.push_op(Op::Rand),
                        1 => {
                            self.emit_expr(&args[0])?;
                            self.push_op(Op::HashF);
                        }
                        2 => {
                            self.emit_expr(&args[0])?;
                            self.emit_expr(&args[1])?;
                            self.push_op(Op::RandBetween);
                        }
                        _ => return Err("rand takes 0, 1, or 2 args".to_string()),
                    }
                    return Ok(());
                }

                for a in args {
                    self.emit_expr(a)?;
                }

                match name.as_str() {
                    "abs" => self.push_op(Op::AbsF),
                    "sin" => self.push_op(Op::Sin),
                    "cos" => self.push_op(Op::Cos),
                    "tan" => self.push_op(Op::Tan),
                    "asin" => self.push_op(Op::Asin),
                    "acos" => self.push_op(Op::Acos),
                    "atan" => self.push_op(Op::Atan),
                    "atan2" => self.push_op(Op::Atan2),
                    "sqrt" => self.push_op(Op::Sqrt),
                    "pow" => self.push_op(Op::Pow),
                    "exp" => self.push_op(Op::Exp),
                    "log" => self.push_op(Op::Log),
                    "log2" => self.push_op(Op::Log2),
                    "floor" => self.push_op(Op::Floor),
                    "ceil" => self.push_op(Op::Ceil),
                    "round" => self.push_op(Op::Round),
                    "fract" => self.push_op(Op::Fract),
                    "min" => self.push_op(Op::Min2),
                    "max" => self.push_op(Op::Max2),
                    "clamp" => self.push_op(Op::Clamp),
                    "mix" => {
                        if args.len() == 3 {
                            self.push_op(Op::Mix)
                        } else {
                            return Err("mix needs 3 args".to_string());
                        }
                    }
                    "step" => self.push_op(Op::Step),
                    "smoothstep" => self.push_op(Op::Smoothstep),
                    "sign" => self.push_op(Op::Sign),
                    "length" => self.push_op(Op::Length2),
                    "rgb" => self.push_op(Op::Rgb),
                    "rgba" => self.push_op(Op::Rgba),
                    "hsl" => self.push_op(Op::Hsl),
                    "hsv" => self.push_op(Op::Hsv),
                    "gray" => self.push_op(Op::Gray),
                    "mixc" => self.push_op(Op::Mix2C),
                    "is_space" => self.push_op(Op::IsSpace),
                    "is_digit" => self.push_op(Op::IsDigit),
                    "is_alpha" => self.push_op(Op::IsAlpha),
                    "is_upper" => self.push_op(Op::IsUpper),
                    "is_lower" => self.push_op(Op::IsLower),
                    _ => {
                        let idx = self
                            .fn_names
                            .iter()
                            .position(|n| n == name)
                            .ok_or_else(|| format!("unknown fn {name}"))?
                            as u8;
                        self.push_op(Op::Call);
                        self.push_u8(idx);
                        self.push_u8(args.len() as u8);
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_stmts(&mut self, stmts: &[Stmt]) -> CompileResult<()> {
        for s in stmts {
            self.emit_stmt(s)?;
        }
        Ok(())
    }

    fn patch_jump_to_here(&mut self, patch_pos: usize) {
        let off = self.code.len() as i16 - (patch_pos as i16 + 2);
        let bytes = off.to_le_bytes();
        self.code[patch_pos] = bytes[0];
        self.code[patch_pos + 1] = bytes[1];
    }

    fn patch_jump_to_target(&mut self, patch_pos: usize, target: i16) {
        let off = target - (patch_pos as i16 + 2);
        let bytes = off.to_le_bytes();
        self.code[patch_pos] = bytes[0];
        self.code[patch_pos + 1] = bytes[1];
    }

    fn emit_return_vals(&mut self, vals: &[Expr]) -> CompileResult<()> {
        match self.ret_type {
            ReturnType::Color => {
                if vals.len() != 1 {
                    return Err("color return requires exactly 1 value".to_string());
                }
                let t = self.infer_expr_type(&vals[0])?;
                if t == ValueType::Scalar {
                    return Err("color return requires a color expression".to_string());
                }
                self.emit_expr(&vals[0])?;
            }
            ReturnType::Char => {
                if vals.len() != 1 {
                    return Err("char return requires exactly 1 value".to_string());
                }
                self.emit_expr(&vals[0])?;
            }
            ReturnType::CharFg => {
                if vals.len() != 2 {
                    return Err("(char,color) return requires exactly 2 values".to_string());
                }
                let fg_t = self.infer_expr_type(&vals[1])?;
                if fg_t == ValueType::Scalar {
                    return Err("second return value must be a color".to_string());
                }
                self.emit_expr(&vals[0])?;
                self.emit_expr(&vals[1])?;
                self.push_op(Op::MakeGlyphCharFg);
            }
            ReturnType::FgBg => {
                if vals.len() != 2 {
                    return Err("(color,color) return requires exactly 2 values".to_string());
                }
                let fg_t = self.infer_expr_type(&vals[0])?;
                let bg_t = self.infer_expr_type(&vals[1])?;
                if fg_t == ValueType::Scalar || bg_t == ValueType::Scalar {
                    return Err("both return values must be colors".to_string());
                }
                self.emit_expr(&vals[0])?;
                self.emit_expr(&vals[1])?;
                self.push_op(Op::MakeGlyphFgBg);
            }
            ReturnType::CharFgBg => {
                if vals.len() != 3 {
                    return Err("(char,color,color) return requires exactly 3 values".to_string());
                }
                let fg_t = self.infer_expr_type(&vals[1])?;
                let bg_t = self.infer_expr_type(&vals[2])?;
                if fg_t == ValueType::Scalar || bg_t == ValueType::Scalar {
                    return Err("foreground/background return values must be colors".to_string());
                }
                self.emit_expr(&vals[0])?;
                self.emit_expr(&vals[1])?;
                self.emit_expr(&vals[2])?;
                self.push_op(Op::MakeGlyphCharFgBg);
            }
        }
        self.push_op(Op::Ret);
        Ok(())
    }

    fn emit_stmt(&mut self, s: &Stmt) -> CompileResult<()> {
        match s {
            Stmt::Let(name, e) => {
                let t = self.infer_expr_type(e)?;
                self.emit_expr(e)?;
                let slot = self.local(name);
                self.push_op(Op::Store);
                self.push_u8(slot);
                self.local_types.insert(name.clone(), t);
            }
            Stmt::Assign(name, e) => {
                let t = self.infer_expr_type(e)?;
                if let Some(prev) = self.local_types.get(name).copied()
                    && prev != ValueType::Unknown
                    && t != ValueType::Unknown
                    && prev != t
                {
                    return Err(format!("type mismatch in assignment to {}", name));
                }
                self.emit_expr(e)?;
                let slot = self.local(name);
                self.push_op(Op::Store);
                self.push_u8(slot);
                self.local_types.insert(name.clone(), t);
            }
            Stmt::ReturnVals(vals) => {
                self.emit_return_vals(vals)?;
            }
            Stmt::If(cond, then_b, else_b) => {
                self.emit_expr(cond)?;
                self.push_op(Op::JmpZ);
                let patch_else = self.code.len();
                self.push_i16(0);
                self.emit_stmts(then_b)?;
                if !else_b.is_empty() {
                    self.push_op(Op::Jmp);
                    let patch_end = self.code.len();
                    self.push_i16(0);
                    self.patch_jump_to_here(patch_else);
                    self.emit_stmts(else_b)?;
                    self.patch_jump_to_here(patch_end);
                } else {
                    self.patch_jump_to_here(patch_else);
                }
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len() as i16;
                self.loops.push(LoopCtx::default());

                self.emit_expr(cond)?;
                self.push_op(Op::JmpZ);
                let patch_exit = self.code.len();
                self.push_i16(0);

                self.emit_stmts(body)?;

                if let Some(loop_ctx) = self.loops.last_mut() {
                    let patches = std::mem::take(&mut loop_ctx.continue_patches);
                    for p in patches {
                        self.patch_jump_to_target(p, loop_start);
                    }
                }

                self.push_op(Op::Jmp);
                let back = loop_start - (self.code.len() as i16 + 2);
                self.push_i16(back);

                self.patch_jump_to_here(patch_exit);
                let end_pos = self.code.len() as i16;
                if let Some(loop_ctx) = self.loops.pop() {
                    for p in loop_ctx.break_patches {
                        self.patch_jump_to_target(p, end_pos);
                    }
                }
            }
            Stmt::For(var, lo, hi, body) => {
                self.emit_expr(lo)?;
                let slot = self.local(var);
                self.push_op(Op::Store);
                self.push_u8(slot);
                self.local_types.insert(var.clone(), ValueType::Scalar);

                let loop_start = self.code.len() as i16;
                self.loops.push(LoopCtx::default());

                self.push_op(Op::Load);
                self.push_u8(slot);
                self.emit_expr(hi)?;
                self.push_op(Op::LtF);
                self.push_op(Op::JmpZ);
                let patch_exit = self.code.len();
                self.push_i16(0);

                self.emit_stmts(body)?;

                let continue_target = self.code.len() as i16;
                if let Some(loop_ctx) = self.loops.last_mut() {
                    let patches = std::mem::take(&mut loop_ctx.continue_patches);
                    for p in patches {
                        self.patch_jump_to_target(p, continue_target);
                    }
                }

                self.push_op(Op::Load);
                self.push_u8(slot);
                self.push_op(Op::PushF);
                self.push_f32(1.0);
                self.push_op(Op::AddF);
                self.push_op(Op::Store);
                self.push_u8(slot);
                self.push_op(Op::Jmp);
                let back = loop_start - (self.code.len() as i16 + 2);
                self.push_i16(back);

                self.patch_jump_to_here(patch_exit);
                let end_pos = self.code.len() as i16;
                if let Some(loop_ctx) = self.loops.pop() {
                    for p in loop_ctx.break_patches {
                        self.patch_jump_to_target(p, end_pos);
                    }
                }
            }
            Stmt::Break => {
                if self.loops.is_empty() {
                    return Err("break used outside of a loop".to_string());
                }
                self.push_op(Op::Jmp);
                let patch = self.code.len();
                self.push_i16(0);
                if let Some(loop_ctx) = self.loops.last_mut() {
                    loop_ctx.break_patches.push(patch);
                }
            }
            Stmt::Continue => {
                if self.loops.is_empty() {
                    return Err("continue used outside of a loop".to_string());
                }
                self.push_op(Op::Jmp);
                let patch = self.code.len();
                self.push_i16(0);
                if let Some(loop_ctx) = self.loops.last_mut() {
                    loop_ctx.continue_patches.push(patch);
                }
            }
            Stmt::Expr(e) => {
                self.emit_expr(e)?;
                self.push_op(Op::Pop);
            }
        }
        Ok(())
    }
}

fn fold_expr(e: &Expr) -> Expr {
    match e {
        Expr::Num(_) | Expr::Var(_) => e.clone(),
        Expr::Field(inner, f) => Expr::Field(Box::new(fold_expr(inner)), f.clone()),
        Expr::UnOp(op, inner) => {
            let fi = fold_expr(inner);
            if let Expr::Num(n) = fi {
                match op {
                    UnOp::Neg => Expr::Num(-n),
                    UnOp::Not => Expr::Num(if n == 0.0 { 1.0 } else { 0.0 }),
                }
            } else {
                Expr::UnOp(*op, Box::new(fi))
            }
        }
        Expr::BinOp(l, op, r) => {
            let fl = fold_expr(l);
            let fr = fold_expr(r);
            if let (Expr::Num(a), Expr::Num(b)) = (&fl, &fr) {
                let v = match op {
                    BinOp::Add => Some(a + b),
                    BinOp::Sub => Some(a - b),
                    BinOp::Mul => Some(a * b),
                    BinOp::Div => Some(if *b == 0.0 { 0.0 } else { a / b }),
                    BinOp::Mod => Some(a % b),
                    BinOp::Eq => Some(if a == b { 1.0 } else { 0.0 }),
                    BinOp::Ne => Some(if a != b { 1.0 } else { 0.0 }),
                    BinOp::Lt => Some(if a < b { 1.0 } else { 0.0 }),
                    BinOp::Gt => Some(if a > b { 1.0 } else { 0.0 }),
                    BinOp::Le => Some(if a <= b { 1.0 } else { 0.0 }),
                    BinOp::Ge => Some(if a >= b { 1.0 } else { 0.0 }),
                    BinOp::And => Some(if *a != 0.0 && *b != 0.0 { 1.0 } else { 0.0 }),
                    BinOp::Or => Some(if *a != 0.0 || *b != 0.0 { 1.0 } else { 0.0 }),
                };
                if let Some(n) = v {
                    return Expr::Num(n);
                }
            }
            Expr::BinOp(Box::new(fl), *op, Box::new(fr))
        }
        Expr::Call(name, args) => {
            let n_args = args.iter().map(fold_expr).collect();
            Expr::Call(name.clone(), n_args)
        }
    }
}

fn optimize_stmts(stmts: &[Stmt]) -> Vec<Stmt> {
    let mut out = Vec::new();
    let mut terminated = false;
    for s in stmts {
        if terminated {
            break;
        }
        let ns = match s {
            Stmt::Let(n, e) => Stmt::Let(n.clone(), fold_expr(e)),
            Stmt::Assign(n, e) => Stmt::Assign(n.clone(), fold_expr(e)),
            Stmt::ReturnVals(vals) => {
                terminated = true;
                Stmt::ReturnVals(vals.iter().map(fold_expr).collect())
            }
            Stmt::If(cond, then_b, else_b) => {
                let c = fold_expr(cond);
                if let Expr::Num(n) = c {
                    if n != 0.0 {
                        let folded = optimize_stmts(then_b);
                        for fs in folded {
                            out.push(fs);
                        }
                        continue;
                    }
                    let folded = optimize_stmts(else_b);
                    for fs in folded {
                        out.push(fs);
                    }
                    continue;
                }
                Stmt::If(c, optimize_stmts(then_b), optimize_stmts(else_b))
            }
            Stmt::While(cond, body) => {
                let c = fold_expr(cond);
                if let Expr::Num(n) = c
                    && n == 0.0
                {
                    continue;
                }
                Stmt::While(c, optimize_stmts(body))
            }
            Stmt::For(v, lo, hi, body) => Stmt::For(
                v.clone(),
                fold_expr(lo),
                fold_expr(hi),
                optimize_stmts(body),
            ),
            Stmt::Break => {
                terminated = true;
                Stmt::Break
            }
            Stmt::Continue => {
                terminated = true;
                Stmt::Continue
            }
            Stmt::Expr(e) => Stmt::Expr(fold_expr(e)),
        };
        out.push(ns);
    }
    out
}

fn optimize_fns(fns: &[FnDef]) -> Vec<FnDef> {
    fns.iter()
        .map(|f| FnDef {
            name: f.name.clone(),
            params: f.params.clone(),
            body: optimize_stmts(&f.body),
            ret_type: f.ret_type,
        })
        .collect()
}

/// Compile parsed functions to bytecode.
pub fn compile_fns(fns: &[FnDef]) -> CompileResult<CompiledShader> {
    let optimized = optimize_fns(fns);
    let fn_names: Vec<String> = optimized.iter().map(|f| f.name.clone()).collect();
    let mut compiled = Vec::new();

    for f in &optimized {
        let mut em = Emitter {
            code: Vec::new(),
            consts: Vec::new(),
            locals: HashMap::new(),
            local_types: HashMap::new(),
            local_count: 0,
            fn_names: &fn_names,
            ret_type: f.ret_type,
            loops: Vec::new(),
        };

        for (idx, p) in f.params.iter().enumerate() {
            em.local(p);
            let pty = if idx == 8 {
                ValueType::Color
            } else {
                ValueType::Scalar
            };
            em.local_types.insert(p.clone(), pty);
        }

        if em.local_count < 11 {
            em.local_count = 11;
        }

        em.emit_stmts(&f.body)?;

        em.push_op(Op::PushF);
        em.push_f32(0.0);
        em.push_op(Op::Ret);

        compiled.push(CompiledFn {
            name: f.name.clone(),
            params: f.params.len() as u8,
            code: em.code,
            const_floats: em.consts,
        });
    }

    let entry = fn_names.iter().position(|n| n == "main").unwrap_or(0);
    let entry_ret = optimized
        .get(entry)
        .map(|f| f.ret_type)
        .unwrap_or(ReturnType::Color);

    Ok(CompiledShader {
        fns: compiled,
        entry,
        entry_ret,
    })
}

fn preprocess_includes_inner(
    src: &str,
    base: &Path,
    visited: &mut HashSet<PathBuf>,
) -> CompileResult<String> {
    let mut out = String::new();
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("include ") {
            let rest = trimmed[8..].trim();
            let (path_text, ok) = if let Some(after_q) = rest.strip_prefix('"') {
                if let Some(end) = after_q.find('"') {
                    (&after_q[..end], true)
                } else {
                    ("", false)
                }
            } else {
                ("", false)
            };
            if !ok || path_text.is_empty() {
                return Err(format!("bad include directive: {}", line));
            }

            let include_path = base.join(path_text);
            let canon = include_path.canonicalize().unwrap_or(include_path.clone());
            if visited.contains(&canon) {
                continue;
            }
            visited.insert(canon.clone());

            let content = fs::read_to_string(&canon)
                .map_err(|e| format!("include read error ({}): {}", canon.display(), e))?;
            let next_base = canon.parent().unwrap_or(base);
            let expanded = preprocess_includes_inner(&content, next_base, visited)?;
            out.push_str(&expanded);
            if !out.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(out)
}

fn preprocess_includes(src: &str) -> CompileResult<String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cwd error: {}", e))?;
    let mut visited = HashSet::new();
    preprocess_includes_inner(src, &cwd, &mut visited)
}

/// Compile TSL source to a `CompiledShader`.
pub fn compile(src: &str) -> Result<CompiledShader, String> {
    let expanded = preprocess_includes(src)?;
    let fns = parse(&expanded)?;
    compile_fns(&fns)
}
