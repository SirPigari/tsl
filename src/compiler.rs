use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

type CompileResult<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct CompileDiagnostic {
    pub message: String,
    pub span: Option<Span>,
    pub notes: Vec<String>,
}

impl CompileDiagnostic {
    fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    fn to_compiler_string(&self) -> String {
        let mut out = if let Some(span) = self.span {
            format!(
                "compile error at byte {}..{}: {}",
                span.start, span.end, self.message
            )
        } else {
            format!("compile error: {}", self.message)
        };
        for n in &self.notes {
            out.push_str("\n  note: ");
            out.push_str(n);
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub tok: Token,
    pub span: Span,
}

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
    Extern,
    True,
    False,
    Break,
    Continue,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Question,
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
        Token::Extern => "extern",
        Token::True => "true",
        Token::False => "false",
        Token::Break => "break",
        Token::Continue => "continue",
        Token::LParen => "(",
        Token::RParen => ")",
        Token::LBrace => "{",
        Token::RBrace => "}",
        Token::LBracket => "[",
        Token::RBracket => "]",
        Token::Comma => ",",
        Token::Semicolon => ";",
        Token::Colon => ":",
        Token::Question => "?",
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

pub fn lex(src: &str) -> Vec<SpannedToken> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut out: Vec<SpannedToken> = Vec::new();
    let mut push = |tok: Token, start: usize, end: usize| {
        out.push(SpannedToken {
            tok,
            span: Span { start, end },
        });
    };
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
                let start_q = i;
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
                push(Token::Str(s), start_q, i);
            }
            b'(' => {
                push(Token::LParen, i, i + 1);
                i += 1;
            }
            b')' => {
                push(Token::RParen, i, i + 1);
                i += 1;
            }
            b'{' => {
                push(Token::LBrace, i, i + 1);
                i += 1;
            }
            b'}' => {
                push(Token::RBrace, i, i + 1);
                i += 1;
            }
            b'[' => {
                push(Token::LBracket, i, i + 1);
                i += 1;
            }
            b']' => {
                push(Token::RBracket, i, i + 1);
                i += 1;
            }
            b',' => {
                push(Token::Comma, i, i + 1);
                i += 1;
            }
            b';' => {
                push(Token::Semicolon, i, i + 1);
                i += 1;
            }
            b':' => {
                push(Token::Colon, i, i + 1);
                i += 1;
            }
            b'?' => {
                push(Token::Question, i, i + 1);
                i += 1;
            }
            b'%' => {
                push(Token::Percent, i, i + 1);
                i += 1;
            }
            b'+' => {
                push(Token::Plus, i, i + 1);
                i += 1;
            }
            b'*' => {
                push(Token::Star, i, i + 1);
                i += 1;
            }
            b'/' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                } else {
                    push(Token::Slash, i, i + 1);
                    i += 1;
                }
            }
            b'-' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                    push(Token::Arrow, i, i + 2);
                    i += 2;
                } else {
                    push(Token::Minus, i, i + 1);
                    i += 1;
                }
            }
            b'.' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                    push(Token::DotDot, i, i + 2);
                    i += 2;
                } else {
                    push(Token::Dot, i, i + 1);
                    i += 1;
                }
            }
            b'=' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    push(Token::EqEq, i, i + 2);
                    i += 2;
                } else {
                    push(Token::Eq, i, i + 1);
                    i += 1;
                }
            }
            b'!' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    push(Token::BangEq, i, i + 2);
                    i += 2;
                } else {
                    push(Token::Bang, i, i + 1);
                    i += 1;
                }
            }
            b'<' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    push(Token::LtEq, i, i + 2);
                    i += 2;
                } else {
                    push(Token::Lt, i, i + 1);
                    i += 1;
                }
            }
            b'>' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                    push(Token::GtEq, i, i + 2);
                    i += 2;
                } else {
                    push(Token::Gt, i, i + 1);
                    i += 1;
                }
            }
            b'&' => {
                let start = i;
                if i + 1 < bytes.len() && bytes[i + 1] == b'&' {
                    i += 1;
                }
                push(Token::And, start, i + 1);
                i += 1;
            }
            b'|' => {
                let start = i;
                if i + 1 < bytes.len() && bytes[i + 1] == b'|' {
                    i += 1;
                }
                push(Token::Or, start, i + 1);
                i += 1;
            }
            b'\'' => {
                let start = i;
                if i + 2 < bytes.len() && bytes[i + 1] != b'\\' && bytes[i + 2] == b'\'' {
                    push(Token::Num(bytes[i + 1] as f32), start, i + 3);
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
                    push(Token::Num(ch as u32 as f32), start, i + 4);
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
                push(Token::Num(v), start, i);
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let s = &src[start..i];
                push(
                    match s {
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "end" => Token::End,
                    "do" => Token::Do,
                    "while" => Token::While,
                    "for" => Token::For,
                    "true" => Token::True,
                    "false" => Token::False,
                    "let" => Token::Let,
                    "extern" => Token::Extern,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    _ => Token::Ident(s.to_string()),
                },
                    start,
                    i,
                );
            }
            _ => {
                i += 1;
            }
        }
    }
    push(Token::Eof, src.len(), src.len());
    out
}

#[derive(Debug, Clone)]
pub enum Expr {
    Num(f32),
    Bool(bool),
    Var(String),
    ArrayLit(Vec<Expr>),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
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
    AssignIndex(String, Expr, Expr),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamQualifier {
    In,
    Out,
    InOut,
    Const,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSpec {
    Float,
    Int,
    Bool,
    Char,
    Color,
    Vec(usize),
    Mat(usize),
    Array(Box<TypeSpec>, Option<usize>),
}

#[derive(Debug, Clone)]
pub struct ParamDecl {
    pub name: String,
    pub ty: TypeSpec,
    pub qual: Vec<ParamQualifier>,
}

fn value_type_name(ty: &ValueType) -> String {
    match ty {
        ValueType::Scalar => "scalar".to_string(),
        ValueType::Color => "color".to_string(),
        ValueType::Bool => "bool".to_string(),
        ValueType::Vector(n) => format!("vec{}", n),
        ValueType::Matrix(n) => format!("matrix{}", n),
        ValueType::ArrayScalar => "float[]".to_string(),
        ValueType::ArrayColor => "color[]".to_string(),
        ValueType::ArrayBool => "bool[]".to_string(),
        ValueType::ArrayVector(n) => format!("vec{}[]", n),
        ValueType::ArrayMatrix(n) => format!("matrix{}[]", n),
        ValueType::ArrayUnknown => "unknown[]".to_string(),
        ValueType::Unknown => "unknown".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternType {
    Number,
    Color,
    Char,
    Bool,
    CharFg,
    FgBg,
    CharFgBg,
}

#[derive(Debug, Clone)]
pub enum ExternDefault {
    Number(f32),
    Color([f32; 3]),
    Char(char),
    Bool(bool),
    CharFg(char, [f32; 3]),
    FgBg([f32; 3], [f32; 3]),
    CharFgBg(char, [f32; 3], [f32; 3]),
}

#[derive(Debug, Clone)]
pub struct ExternDecl {
    pub name: String,
    pub ty: ExternType,
    pub default: Option<ExternDefault>,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub externs: Vec<ExternDecl>,
    pub fns: Vec<FnDef>,
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<ParamDecl>,
    pub body: Vec<Stmt>,
    pub ret_type: ReturnType,
}

struct Parser {
    toks: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &SpannedToken {
        &self.toks[self.pos]
    }

    fn peek_tok(&self) -> &Token {
        &self.toks[self.pos].tok
    }

    fn next(&mut self) -> SpannedToken {
        let t = self.toks[self.pos].clone();
        self.pos += 1;
        t
    }

    fn eat(&mut self, t: &Token) -> bool {
        if std::mem::discriminant(self.peek_tok()) == std::mem::discriminant(t) {
            self.next();
            true
        } else {
            false
        }
    }

    fn eat_ident_word(&mut self, word: &str) -> bool {
        match self.peek_tok() {
            Token::Ident(s) if s == word => {
                self.next();
                true
            }
            _ => false,
        }
    }

    fn err<T>(&self, msg: &str) -> CompileResult<T> {
        let diag = CompileDiagnostic::new(
            format!(
                "parse error near token {} ({}): {}",
                self.pos,
                token_name(self.peek_tok()),
                msg
            ),
            Some(self.peek().span),
        )
        .with_note("add or adjust syntax near this token");
        Err(diag.to_compiler_string())
    }

    fn expect(&mut self, t: &Token, ctx: &str) -> CompileResult<()> {
        if self.eat(t) {
            Ok(())
        } else {
            self.err(&format!("expected {} {}", token_name(t), ctx))
        }
    }

    fn expect_ident(&mut self) -> CompileResult<String> {
        let t = self.next();
        match t.tok {
            Token::Ident(s) => Ok(s),
            other => {
                let diag = CompileDiagnostic::new(
                    format!("expected identifier, got {}", token_name(&other)),
                    Some(t.span),
                );
                Err(diag.to_compiler_string())
            }
        }
    }

    fn parse_ret_type(&mut self) -> CompileResult<ReturnType> {
        if self.eat(&Token::LParen) {
            let mut parts = Vec::new();
            while !matches!(self.peek_tok(), Token::RParen | Token::Eof) {
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

    fn parse_extern_type(&mut self) -> CompileResult<ExternType> {
        if self.eat(&Token::LParen) {
            let mut parts = Vec::new();
            while !matches!(self.peek_tok(), Token::RParen | Token::Eof) {
                parts.push(self.expect_ident()?.to_lowercase());
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen, "to close extern tuple type")?;
            return match parts.as_slice() {
                [a, b] if a == "char" && b == "color" => Ok(ExternType::CharFg),
                [a, b] if a == "color" && b == "color" => Ok(ExternType::FgBg),
                [a, b, c] if a == "char" && b == "color" && c == "color" => {
                    Ok(ExternType::CharFgBg)
                }
                _ => {
                    Err("unsupported extern tuple type; use (char,color), (color,color), or (char,color,color)".to_string())
                }
            };
        }

        match self.expect_ident()?.to_lowercase().as_str() {
            "number" | "float" | "scalar" => Ok(ExternType::Number),
            "color" => Ok(ExternType::Color),
            "char" => Ok(ExternType::Char),
            "bool" => Ok(ExternType::Bool),
            _ => Err(
                "unsupported extern type; use number, color, char, bool, or tuple variants"
                    .to_string(),
            ),
        }
    }

    fn parse_bool_default(&mut self) -> CompileResult<bool> {
        match self.peek_tok() {
            Token::True => {
                self.next();
                return Ok(true);
            }
            Token::False => {
                self.next();
                return Ok(false);
            }
            _ => {}
        }
        if let Token::Ident(s) = self.peek_tok().clone() {
            let low = s.to_lowercase();
            if low == "true" {
                self.next();
                return Ok(true);
            }
            if low == "false" {
                self.next();
                return Ok(false);
            }
        }
        Ok(self.parse_number_default()? != 0.0)
    }

    fn parse_number_default(&mut self) -> CompileResult<f32> {
        let e = self.parse_expr();
        match fold_expr(&e) {
            Expr::Num(n) => Ok(n),
            _ => Err("extern number default must be a constant numeric expression".to_string()),
        }
    }

    fn parse_char_default(&mut self) -> CompileResult<char> {
        let n = self.parse_number_default()?;
        let code = n as u32;
        char::from_u32(code).ok_or_else(|| "extern char default is not a valid Unicode codepoint".to_string())
    }

    fn parse_color_default(&mut self) -> CompileResult<[f32; 3]> {
        let e = self.parse_expr();
        eval_const_color(&e)
            .ok_or_else(|| "extern color default must be a constant color expression".to_string())
    }

    fn parse_extern_default(&mut self, ty: ExternType) -> CompileResult<ExternDefault> {
        match ty {
            ExternType::Number => Ok(ExternDefault::Number(self.parse_number_default()?)),
            ExternType::Color => Ok(ExternDefault::Color(self.parse_color_default()?)),
            ExternType::Char => Ok(ExternDefault::Char(self.parse_char_default()?)),
            ExternType::Bool => Ok(ExternDefault::Bool(self.parse_bool_default()?)),
            ExternType::CharFg => {
                self.expect(&Token::LParen, "before (char,color) extern default")?;
                let ch = self.parse_char_default()?;
                self.expect(&Token::Comma, "between char and color default")?;
                let fg = self.parse_color_default()?;
                self.expect(&Token::RParen, "after (char,color) extern default")?;
                Ok(ExternDefault::CharFg(ch, fg))
            }
            ExternType::FgBg => {
                self.expect(&Token::LParen, "before (color,color) extern default")?;
                let fg = self.parse_color_default()?;
                self.expect(&Token::Comma, "between foreground and background default")?;
                let bg = self.parse_color_default()?;
                self.expect(&Token::RParen, "after (color,color) extern default")?;
                Ok(ExternDefault::FgBg(fg, bg))
            }
            ExternType::CharFgBg => {
                self.expect(&Token::LParen, "before (char,color,color) extern default")?;
                let ch = self.parse_char_default()?;
                self.expect(&Token::Comma, "between char and foreground default")?;
                let fg = self.parse_color_default()?;
                self.expect(&Token::Comma, "between foreground and background default")?;
                let bg = self.parse_color_default()?;
                self.expect(&Token::RParen, "after (char,color,color) extern default")?;
                Ok(ExternDefault::CharFgBg(ch, fg, bg))
            }
        }
    }

    fn parse_extern_decl(&mut self) -> CompileResult<ExternDecl> {
        self.expect(&Token::Extern, "before extern declaration")?;
        let ty = self.parse_extern_type()?;
        let name = self.expect_ident()?;
        let default = if self.eat(&Token::Eq) {
            Some(self.parse_extern_default(ty)?)
        } else {
            None
        };
        self.eat(&Token::Semicolon);
        Ok(ExternDecl { name, ty, default })
    }

    fn parse_type_spec(&mut self, allow_default_float: bool) -> CompileResult<TypeSpec> {
        let mut ty = match self.peek_tok() {
            Token::Ident(s) if s == "float" || s == "number" => {
                self.next();
                TypeSpec::Float
            }
            Token::Ident(s) if s == "int" => {
                self.next();
                TypeSpec::Int
            }
            Token::Ident(s) if s == "bool" => {
                self.next();
                TypeSpec::Bool
            }
            Token::Ident(s) if s == "char" => {
                self.next();
                TypeSpec::Char
            }
            Token::Ident(s) if s == "color" => {
                self.next();
                TypeSpec::Color
            }
            Token::Ident(s) if s.starts_with("vec") => {
                let n = s[3..].parse::<usize>().unwrap_or(0);
                if (2..=4).contains(&n) {
                    self.next();
                    TypeSpec::Vec(n)
                } else {
                    return self.err("expected vec2, vec3, or vec4");
                }
            }
            Token::Ident(s) if s.starts_with("matrix") => {
                let n = s[6..].parse::<usize>().unwrap_or(0);
                if (2..=4).contains(&n) {
                    self.next();
                    TypeSpec::Mat(n)
                } else {
                    return self.err("expected matrix2, matrix3, or matrix4");
                }
            }
            _ => {
                if allow_default_float {
                    TypeSpec::Float
                } else {
                    return self.err("expected parameter type after qualifier");
                }
            }
        };

        while self.eat(&Token::LBracket) {
            let len = match self.peek_tok() {
                Token::Num(n) => {
                    let raw = *n;
                    self.next();
                    if raw < 0.0 || raw.fract() != 0.0 {
                        return self.err("array size must be a non-negative integer literal");
                    }
                    Some(raw as usize)
                }
                _ => None,
            };
            self.expect(&Token::RBracket, "to close array type")?;
            ty = TypeSpec::Array(Box::new(ty), len);
        }

        Ok(ty)
    }

    fn parse_fn(&mut self) -> CompileResult<FnDef> {
        self.expect(&Token::Fn, "before function name")?;
        let name = self.expect_ident()?;
        self.expect(&Token::LParen, "after function name")?;

        let mut params = Vec::new();
        while !matches!(self.peek_tok(), Token::RParen | Token::Eof) {
            let mut qual = Vec::new();
            loop {
                let next = match self.peek_tok() {
                    Token::In => Some(ParamQualifier::In),
                    Token::Ident(s) if s == "in" => Some(ParamQualifier::In),
                    Token::Ident(s) if s == "out" => Some(ParamQualifier::Out),
                    Token::Ident(s) if s == "inout" => Some(ParamQualifier::InOut),
                    Token::Ident(s) if s == "const" => Some(ParamQualifier::Const),
                    _ => None,
                };
                if let Some(q) = next {
                    self.next();
                    qual.push(q);
                } else {
                    break;
                }
            }

            let ty = self.parse_type_spec(qual.is_empty())?;

            let name = self.expect_ident()?;
            params.push(ParamDecl { name, ty, qual });
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
        while !matches!(self.peek_tok(), Token::End | Token::Else | Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&Token::End, "to close block")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> CompileResult<Stmt> {
        let stmt = match self.peek_tok().clone() {
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
                    while !matches!(self.peek_tok(), Token::RParen | Token::Eof) {
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
                while !matches!(self.peek_tok(), Token::Else | Token::End | Token::Eof) {
                    then_b.push(self.parse_stmt()?);
                }
                let else_b = if self.eat(&Token::Else) {
                    let mut eb = Vec::new();
                    while !matches!(self.peek_tok(), Token::End | Token::Eof) {
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
                if !(self.eat(&Token::In) || self.eat_ident_word("in")) {
                    return self.err("expected in in for loop");
                }
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
                if self.eat(&Token::LBracket) {
                    let idx = self.parse_expr();
                    self.expect(&Token::RBracket, "after index expression")?;
                    self.expect(&Token::Eq, "after indexed lvalue")?;
                    let e = self.parse_expr();
                    self.eat(&Token::Semicolon);
                    Stmt::AssignIndex(name, idx, e)
                } else if self.eat(&Token::Eq) {
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
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Expr {
        let cond = self.parse_or();
        if matches!(self.peek_tok(), Token::Question) {
            self.next();
            let then_e = self.parse_expr();
            if self.eat(&Token::Colon) {
                let else_e = self.parse_expr();
                return Expr::Ternary(Box::new(cond), Box::new(then_e), Box::new(else_e));
            }
        }
        cond
    }

    fn parse_or(&mut self) -> Expr {
        let mut l = self.parse_and();
        while matches!(self.peek_tok(), Token::Or) {
            self.next();
            let r = self.parse_and();
            l = Expr::BinOp(Box::new(l), BinOp::Or, Box::new(r));
        }
        l
    }

    fn parse_and(&mut self) -> Expr {
        let mut l = self.parse_cmp();
        while matches!(self.peek_tok(), Token::And) {
            self.next();
            let r = self.parse_cmp();
            l = Expr::BinOp(Box::new(l), BinOp::And, Box::new(r));
        }
        l
    }

    fn parse_cmp(&mut self) -> Expr {
        let l = self.parse_add();
        match self.peek_tok().clone() {
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
            match self.peek_tok().clone() {
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
            match self.peek_tok().clone() {
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
        match self.peek_tok().clone() {
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
            } else if self.eat(&Token::LBracket) {
                let idx = self.parse_expr();
                self.eat(&Token::RBracket);
                e = Expr::Index(Box::new(e), Box::new(idx));
            } else {
                break;
            }
        }
        e
    }

    fn parse_primary(&mut self) -> Expr {
        match self.peek_tok().clone() {
            Token::Num(n) => {
                self.next();
                Expr::Num(n)
            }
            Token::True => {
                self.next();
                Expr::Bool(true)
            }
            Token::False => {
                self.next();
                Expr::Bool(false)
            }
            Token::LParen => {
                self.next();
                let e = self.parse_expr();
                self.eat(&Token::RParen);
                e
            }
            Token::LBracket => {
                self.next();
                let mut items = Vec::new();
                while !matches!(self.peek_tok(), Token::RBracket | Token::Eof) {
                    items.push(self.parse_expr());
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.eat(&Token::RBracket);
                Expr::ArrayLit(items)
            }
            Token::Ident(name) => {
                self.next();
                if self.eat(&Token::LParen) {
                    let mut args = Vec::new();
                    while !matches!(self.peek_tok(), Token::RParen | Token::Eof) {
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
    Ok(parse_program(src)?.fns)
}

/// Parse source code into a full program AST (extern declarations + functions).
pub fn parse_program(src: &str) -> CompileResult<Program> {
    let toks = lex(src);
    let mut p = Parser { toks, pos: 0 };
    let mut externs = Vec::new();
    let mut fns = Vec::new();
    while !matches!(p.peek_tok(), Token::Eof) {
        if matches!(p.peek_tok(), Token::Extern) {
            externs.push(p.parse_extern_decl()?);
        } else {
            fns.push(p.parse_fn()?);
        }
    }
    Ok(Program { externs, fns })
}

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Op {
    PushF = 0,
    PushC = 1,
    Pop = 2,
    Load = 3,
    Store = 4,
    AddF = 10,
    SubF = 11,
    MulF = 12,
    DivF = 13,
    ModF = 14,
    NegF = 15,
    AbsF = 16,
    AddC = 20,
    SubC = 21,
    MulC = 22,
    DivC = 23,
    MulCF = 24,
    EqF = 30,
    NeF = 31,
    LtF = 32,
    GtF = 33,
    LeF = 34,
    GeF = 35,
    AndF = 36,
    OrF = 37,
    NotF = 38,
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
    Rgb = 70,
    Rgba = 71,
    Hsl = 72,
    Hsv = 73,
    Gray = 74,
    Mix2C = 75,
    GetR = 80,
    GetG = 81,
    GetB = 82,
    IsSpace = 83,
    IsDigit = 84,
    IsAlpha = 85,
    IsUpper = 86,
    IsLower = 87,
    Jmp = 90,
    JmpZ = 91,
    Ret = 99,
    Call = 100,
    Rand = 101,
    RandBetween = 102,
    MakeGlyphCharFg = 103,
    MakeGlyphFgBg = 104,
    MakeGlyphCharFgBg = 105,
    HashF = 106,
    Vec2 = 107,
    Vec3 = 108,
    Vec4 = 109,
    Mat2 = 110,
    Mat3 = 111,
    Mat4 = 112,
    Swizzle = 113,
    Dot = 114,
    Cross = 115,
    Normalize = 116,
    Reflect = 117,
    Refract = 118,
    CallExt = 119,
    ArrayMake = 120,
    ArrayGet = 121,
    ArraySet = 122,
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
    pub externs: Vec<ExternDecl>,
}

const MAGIC: &[u8; 4] = b"CTSL";
const VERSION: u8 = 5;

fn extern_type_tag(ty: ExternType) -> u8 {
    match ty {
        ExternType::Number => 0,
        ExternType::Color => 1,
        ExternType::Char => 2,
        ExternType::Bool => 3,
        ExternType::CharFg => 4,
        ExternType::FgBg => 5,
        ExternType::CharFgBg => 6,
    }
}

fn extern_type_from_tag(tag: u8) -> Option<ExternType> {
    match tag {
        0 => Some(ExternType::Number),
        1 => Some(ExternType::Color),
        2 => Some(ExternType::Char),
        3 => Some(ExternType::Bool),
        4 => Some(ExternType::CharFg),
        5 => Some(ExternType::FgBg),
        6 => Some(ExternType::CharFgBg),
        _ => None,
    }
}

fn write_rgb(out: &mut Vec<u8>, c: [f32; 3]) {
    out.extend_from_slice(&c[0].to_le_bytes());
    out.extend_from_slice(&c[1].to_le_bytes());
    out.extend_from_slice(&c[2].to_le_bytes());
}

fn read_f32_at(data: &[u8], pos: &mut usize) -> Result<f32, &'static str> {
    if *pos + 4 > data.len() {
        return Err("truncated");
    }
    let v = f32::from_le_bytes(data[*pos..*pos + 4].try_into().map_err(|_| "truncated")?);
    *pos += 4;
    Ok(v)
}

fn read_rgb(data: &[u8], pos: &mut usize) -> Result<[f32; 3], &'static str> {
    Ok([
        read_f32_at(data, pos)?,
        read_f32_at(data, pos)?,
        read_f32_at(data, pos)?,
    ])
}

fn write_extern_default(out: &mut Vec<u8>, def: &ExternDefault) {
    match def {
        ExternDefault::Number(v) => out.extend_from_slice(&v.to_le_bytes()),
        ExternDefault::Color(c) => write_rgb(out, *c),
        ExternDefault::Char(ch) => out.extend_from_slice(&(*ch as u32).to_le_bytes()),
        ExternDefault::Bool(v) => out.push(if *v { 1 } else { 0 }),
        ExternDefault::CharFg(ch, fg) => {
            out.extend_from_slice(&(*ch as u32).to_le_bytes());
            write_rgb(out, *fg);
        }
        ExternDefault::FgBg(fg, bg) => {
            write_rgb(out, *fg);
            write_rgb(out, *bg);
        }
        ExternDefault::CharFgBg(ch, fg, bg) => {
            out.extend_from_slice(&(*ch as u32).to_le_bytes());
            write_rgb(out, *fg);
            write_rgb(out, *bg);
        }
    }
}

fn read_extern_default(
    data: &[u8],
    pos: &mut usize,
    ty: ExternType,
) -> Result<ExternDefault, &'static str> {
    match ty {
        ExternType::Number => Ok(ExternDefault::Number(read_f32_at(data, pos)?)),
        ExternType::Color => Ok(ExternDefault::Color(read_rgb(data, pos)?)),
        ExternType::Char => {
            if *pos + 4 > data.len() {
                return Err("truncated");
            }
            let ch = u32::from_le_bytes(data[*pos..*pos + 4].try_into().map_err(|_| "truncated")?);
            *pos += 4;
            Ok(ExternDefault::Char(char::from_u32(ch).unwrap_or('\0')))
        }
        ExternType::Bool => {
            let Some(&b) = data.get(*pos) else {
                return Err("truncated");
            };
            *pos += 1;
            Ok(ExternDefault::Bool(b != 0))
        }
        ExternType::CharFg => {
            if *pos + 4 > data.len() {
                return Err("truncated");
            }
            let ch = u32::from_le_bytes(data[*pos..*pos + 4].try_into().map_err(|_| "truncated")?);
            *pos += 4;
            let fg = read_rgb(data, pos)?;
            Ok(ExternDefault::CharFg(char::from_u32(ch).unwrap_or('\0'), fg))
        }
        ExternType::FgBg => {
            let fg = read_rgb(data, pos)?;
            let bg = read_rgb(data, pos)?;
            Ok(ExternDefault::FgBg(fg, bg))
        }
        ExternType::CharFgBg => {
            if *pos + 4 > data.len() {
                return Err("truncated");
            }
            let ch = u32::from_le_bytes(data[*pos..*pos + 4].try_into().map_err(|_| "truncated")?);
            *pos += 4;
            let fg = read_rgb(data, pos)?;
            let bg = read_rgb(data, pos)?;
            Ok(ExternDefault::CharFgBg(char::from_u32(ch).unwrap_or('\0'), fg, bg))
        }
    }
}

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
        out.extend_from_slice(&(self.externs.len() as u16).to_le_bytes());
        for ext in &self.externs {
            out.push(extern_type_tag(ext.ty));
            let name = ext.name.as_bytes();
            out.push(name.len() as u8);
            out.extend_from_slice(name);
            out.push(if ext.default.is_some() { 1 } else { 0 });
            if let Some(def) = &ext.default {
                write_extern_default(&mut out, def);
            }
        }
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
        if ver != 1 && ver != 2 && ver != 3 && ver != 4 && ver != VERSION {
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

        let mut externs = Vec::new();
        if ver >= 5 {
            if pos + 2 > data.len() {
                return Err("truncated");
            }
            let ext_count = u16::from_le_bytes(data[pos..pos + 2].try_into().map_err(|_| "truncated")?) as usize;
            pos += 2;
            externs.reserve(ext_count);
            for _ in 0..ext_count {
                let Some(&ty_tag) = data.get(pos) else {
                    return Err("truncated");
                };
                pos += 1;
                let ty = extern_type_from_tag(ty_tag).ok_or("bad extern type")?;

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

                let Some(&has_default) = data.get(pos) else {
                    return Err("truncated");
                };
                pos += 1;
                let default = if has_default != 0 {
                    Some(read_extern_default(data, &mut pos, ty)?)
                } else {
                    None
                };
                externs.push(ExternDecl { name, ty, default });
            }
        }

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
            externs,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueType {
    Scalar,
    Color,
    Bool,
    Vector(usize),
    Matrix(usize),
    ArrayScalar,
    ArrayColor,
    ArrayBool,
    ArrayVector(usize),
    ArrayMatrix(usize),
    ArrayUnknown,
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
    readonly_locals: HashSet<String>,
    local_count: u8,
    fn_names: &'a [String],
    fn_param_types: &'a HashMap<String, Vec<ValueType>>,
    fn_param_quals: &'a HashMap<String, Vec<u8>>,
    fn_ret_types: &'a HashMap<String, ValueType>,
    extern_slots: &'a HashMap<String, u8>,
    extern_types: &'a HashMap<String, ValueType>,
    ret_type: ReturnType,
    loops: Vec<LoopCtx>,
}

impl<'a> Emitter<'a> {
    fn infer_binop_type(&self, op: BinOp, lt: ValueType, rt: ValueType) -> CompileResult<ValueType> {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Div => match (lt, rt) {
                (ValueType::Scalar, ValueType::Scalar) => Ok(ValueType::Scalar),
                (ValueType::Color, ValueType::Color) => Ok(ValueType::Color),
                (ValueType::Vector(a), ValueType::Vector(b)) if a == b => Ok(ValueType::Vector(a)),
                (ValueType::Matrix(a), ValueType::Matrix(b)) if a == b => Ok(ValueType::Matrix(a)),
                _ => Err(format!(
                    "type mismatch: {} requires matching scalar/color/vector/matrix operands",
                    match op {
                        BinOp::Add => "+",
                        BinOp::Sub => "-",
                        _ => "/",
                    }
                )),
            },
            BinOp::Mul => match (lt, rt) {
                (ValueType::Scalar, ValueType::Scalar) => Ok(ValueType::Scalar),
                (ValueType::Color, ValueType::Color) => Ok(ValueType::Color),
                (ValueType::Color, ValueType::Scalar) | (ValueType::Scalar, ValueType::Color) => Ok(ValueType::Color),
                (ValueType::Vector(a), ValueType::Vector(b)) if a == b => Ok(ValueType::Vector(a)),
                (ValueType::Vector(a), ValueType::Scalar) | (ValueType::Scalar, ValueType::Vector(a)) => {
                    Ok(ValueType::Vector(a))
                }
                (ValueType::Matrix(a), ValueType::Matrix(b)) if a == b => Ok(ValueType::Matrix(a)),
                (ValueType::Matrix(a), ValueType::Vector(b)) if a == b => Ok(ValueType::Vector(a)),
                (ValueType::Matrix(a), ValueType::Scalar) | (ValueType::Scalar, ValueType::Matrix(a)) => {
                    Ok(ValueType::Matrix(a))
                }
                _ => Err("type mismatch for '*' operands".to_string()),
            },
            BinOp::Mod => {
                if !is_scalar_like(lt) || !is_scalar_like(rt) {
                    return Err("type mismatch: operator requires scalar operands".to_string());
                }
                Ok(ValueType::Scalar)
            }
            BinOp::Eq | BinOp::Ne => {
                if lt != rt || matches!(lt, ValueType::Unknown | ValueType::ArrayUnknown) {
                    return Err("type mismatch: equality requires matching concrete operand types".to_string());
                }
                Ok(ValueType::Bool)
            }
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                if lt != ValueType::Scalar || rt != ValueType::Scalar {
                    return Err("type mismatch: ordered comparisons require scalar operands".to_string());
                }
                Ok(ValueType::Bool)
            }
            BinOp::And | BinOp::Or => {
                if lt != ValueType::Bool || rt != ValueType::Bool {
                    return Err("type mismatch: logical operators require bool operands".to_string());
                }
                Ok(ValueType::Bool)
            }
        }
    }

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
            Expr::Bool(_) => Ok(ValueType::Bool),
            Expr::Var(name) => {
                if let Some(t) = self.local_types.get(name) {
                    Ok(*t)
                } else {
                    Ok(*self.extern_types.get(name).unwrap_or(&ValueType::Unknown))
                }
            }
            Expr::ArrayLit(items) => {
                if items.is_empty() {
                    return Ok(ValueType::ArrayUnknown);
                }
                let elem_t = self.infer_expr_type(&items[0])?;
                for item in items.iter().skip(1) {
                    let t = self.infer_expr_type(item)?;
                    if t != elem_t {
                        return Err("array literal elements must have the same type".to_string());
                    }
                }
                Ok(match elem_t {
                    ValueType::Scalar => ValueType::ArrayScalar,
                    ValueType::Color => ValueType::ArrayColor,
                    ValueType::Bool => ValueType::ArrayBool,
                    ValueType::Vector(n) => ValueType::ArrayVector(n),
                    ValueType::Matrix(n) => ValueType::ArrayMatrix(n),
                    _ => ValueType::ArrayUnknown,
                })
            }
            Expr::Index(base, idx) => {
                let base_t = self.infer_expr_type(base)?;
                let idx_t = self.infer_expr_type(idx)?;
                if idx_t != ValueType::Scalar {
                    return Err("array index must be scalar".to_string());
                }
                array_elem_type(base_t).ok_or_else(|| "indexing requires an array value".to_string())
            }
            Expr::Field(inner, field) => {
                let t = self.infer_expr_type(inner)?;
                swizzle_result_type(t, field)
            }
            Expr::UnOp(op, a) => {
                let t = self.infer_expr_type(a)?;
                match op {
                    UnOp::Neg => match t {
                        ValueType::Scalar | ValueType::Unknown => Ok(ValueType::Scalar),
                        ValueType::Color => Err("unary operator cannot be applied to color".to_string()),
                        ValueType::Vector(n) => Ok(ValueType::Vector(n)),
                        ValueType::Matrix(n) => Ok(ValueType::Matrix(n)),
                        ValueType::Bool
                        | ValueType::ArrayScalar
                        | ValueType::ArrayColor
                        | ValueType::ArrayBool
                        | ValueType::ArrayVector(_)
                        | ValueType::ArrayMatrix(_)
                        | ValueType::ArrayUnknown => {
                            Err("unary operator cannot be applied to this type".to_string())
                        }
                    },
                    UnOp::Not => {
                        if t != ValueType::Bool {
                            Err("logical not requires a bool".to_string())
                        } else {
                            Ok(ValueType::Bool)
                        }
                    }
                }
            }
            Expr::Ternary(cond, then_e, else_e) => {
                let ct = self.infer_expr_type(cond)?;
                if ct != ValueType::Bool {
                    return Err("ternary condition must be bool".to_string());
                }
                let tt = self.infer_expr_type(then_e)?;
                let et = self.infer_expr_type(else_e)?;
                if tt != et {
                    return Err("ternary branches must have the same type".to_string());
                }
                Ok(tt)
            }
            Expr::BinOp(l, op, r) => {
                let lt = self.infer_expr_type(l)?;
                let rt = self.infer_expr_type(r)?;
                self.infer_binop_type(*op, lt, rt)
            }
            Expr::Call(name, args) => {
                let n = name.as_str();
                match n {
                    "vec2" => {
                        if args.len() != 2 {
                            return Err("vec2 requires exactly 2 scalar arguments".to_string());
                        }
                        for arg in args {
                            if !matches!(self.infer_expr_type(arg)?, ValueType::Scalar) {
                                return Err("vec2 arguments must be scalar".to_string());
                            }
                        }
                        return Ok(ValueType::Vector(2));
                    }
                    "vec3" => {
                        if args.len() != 3 {
                            return Err("vec3 requires exactly 3 scalar arguments".to_string());
                        }
                        for arg in args {
                            if !matches!(self.infer_expr_type(arg)?, ValueType::Scalar) {
                                return Err("vec3 arguments must be scalar".to_string());
                            }
                        }
                        return Ok(ValueType::Vector(3));
                    }
                    "vec4" => {
                        if args.len() != 4 {
                            return Err("vec4 requires exactly 4 scalar arguments".to_string());
                        }
                        for arg in args {
                            if !matches!(self.infer_expr_type(arg)?, ValueType::Scalar) {
                                return Err("vec4 arguments must be scalar".to_string());
                            }
                        }
                        return Ok(ValueType::Vector(4));
                    }
                    "matrix2" => {
                        if args.len() != 4 {
                            return Err("matrix2 requires exactly 4 scalar arguments".to_string());
                        }
                        for arg in args {
                            if !matches!(self.infer_expr_type(arg)?, ValueType::Scalar) {
                                return Err("matrix2 arguments must be scalar".to_string());
                            }
                        }
                        return Ok(ValueType::Matrix(2));
                    }
                    "matrix3" => {
                        if args.len() != 9 {
                            return Err("matrix3 requires exactly 9 scalar arguments".to_string());
                        }
                        for arg in args {
                            if !matches!(self.infer_expr_type(arg)?, ValueType::Scalar) {
                                return Err("matrix3 arguments must be scalar".to_string());
                            }
                        }
                        return Ok(ValueType::Matrix(3));
                    }
                    "matrix4" => {
                        if args.len() != 16 {
                            return Err("matrix4 requires exactly 16 scalar arguments".to_string());
                        }
                        for arg in args {
                            if !matches!(self.infer_expr_type(arg)?, ValueType::Scalar) {
                                return Err("matrix4 arguments must be scalar".to_string());
                            }
                        }
                        return Ok(ValueType::Matrix(4));
                    }
                    "dot" => {
                        if args.len() != 2 {
                            return Err("dot requires 2 vector arguments".to_string());
                        }
                        let a = self.infer_expr_type(&args[0])?;
                        let b = self.infer_expr_type(&args[1])?;
                        if vector_len(a).is_none() || vector_len(b).is_none() || vector_len(a) != vector_len(b) {
                            return Err("dot requires matching vector arguments".to_string());
                        }
                        return Ok(ValueType::Scalar);
                    }
                    "cross" => {
                        if args.len() != 2 {
                            return Err("cross requires 2 vector arguments".to_string());
                        }
                        let a = self.infer_expr_type(&args[0])?;
                        let b = self.infer_expr_type(&args[1])?;
                        if vector_len(a) != Some(3) || vector_len(b) != Some(3) {
                            return Err("cross requires vec3 arguments".to_string());
                        }
                        return Ok(ValueType::Vector(3));
                    }
                    "normalize" => {
                        if args.len() != 1 {
                            return Err("normalize requires 1 vector argument".to_string());
                        }
                        let t = self.infer_expr_type(&args[0])?;
                        if vector_len(t).is_none() {
                            return Err("normalize requires a vector argument".to_string());
                        }
                        return Ok(t);
                    }
                    "reflect" => {
                        if args.len() != 2 {
                            return Err("reflect requires 2 vector arguments".to_string());
                        }
                        let a = self.infer_expr_type(&args[0])?;
                        let b = self.infer_expr_type(&args[1])?;
                        if vector_len(a).is_none() || vector_len(a) != vector_len(b) {
                            return Err("reflect requires matching vector arguments".to_string());
                        }
                        return Ok(a);
                    }
                    "refract" => {
                        if args.len() != 3 {
                            return Err("refract requires 3 arguments".to_string());
                        }
                        let a = self.infer_expr_type(&args[0])?;
                        let b = self.infer_expr_type(&args[1])?;
                        let eta = self.infer_expr_type(&args[2])?;
                        if vector_len(a).is_none() || vector_len(a) != vector_len(b) || eta != ValueType::Scalar {
                            return Err("refract requires matching vector args and scalar eta".to_string());
                        }
                        return Ok(a);
                    }
                    _ => {}
                }
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
                if let Some(ret) = self.fn_ret_types.get(name) {
                    return Ok(*ret);
                }
                Ok(ValueType::Unknown)
            }
        }
    }

    fn emit_short_circuit(&mut self, op: BinOp, l: &Expr, r: &Expr) -> CompileResult<()> {
        if self.infer_expr_type(l)? != ValueType::Bool || self.infer_expr_type(r)? != ValueType::Bool {
            return Err("logical operators require bool operands".to_string());
        }
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
            Expr::Bool(v) => {
                self.push_op(Op::PushF);
                self.push_f32(if *v { 1.0 } else { 0.0 });
            }
            Expr::ArrayLit(items) => {
                if items.len() > u8::MAX as usize {
                    return Err("array literal too large (max 255 elements)".to_string());
                }
                for item in items {
                    self.emit_expr(item)?;
                }
                self.push_op(Op::ArrayMake);
                self.push_u8(items.len() as u8);
            }
            Expr::Var(name) => {
                let s = if let Some(&slot) = self.locals.get(name.as_str()) {
                    slot
                } else if let Some(&slot) = self.extern_slots.get(name.as_str()) {
                    slot
                } else {
                    return Err(format!("undef var {name}"));
                };
                self.push_op(Op::Load);
                self.push_u8(s);
            }
            Expr::Index(base, idx) => {
                self.emit_expr(base)?;
                self.emit_expr(idx)?;
                self.push_op(Op::ArrayGet);
            }
            Expr::Field(inner, field) => {
                self.emit_expr(inner)?;
                match field.as_str() {
                    "r" => self.push_op(Op::GetR),
                    "g" => self.push_op(Op::GetG),
                    "b" => self.push_op(Op::GetB),
                    _ => {
                        self.push_op(Op::Swizzle);
                        self.push_u8(field.len() as u8);
                        for ch in field.bytes() {
                            self.push_u8(ch);
                        }
                    }
                }
            }
            Expr::UnOp(op, a) => {
                let t = self.infer_expr_type(a)?;
                match op {
                    UnOp::Neg => {
                        if matches!(t, ValueType::Color) {
                            return Err("type mismatch: unary operator requires non-color operand".to_string());
                        }
                    }
                    UnOp::Not => {
                        if t != ValueType::Bool {
                            return Err("logical not requires a bool".to_string());
                        }
                    }
                }
                self.emit_expr(a)?;
                match op {
                    UnOp::Neg => self.push_op(Op::NegF),
                    UnOp::Not => self.push_op(Op::NotF),
                }
            }
            Expr::Ternary(cond, then_e, else_e) => {
                let ct = self.infer_expr_type(cond)?;
                if ct != ValueType::Bool {
                    return Err("ternary condition must be bool".to_string());
                }
                let tt = self.infer_expr_type(then_e)?;
                let et = self.infer_expr_type(else_e)?;
                if tt != et {
                    return Err("ternary branches must have the same type".to_string());
                }

                self.emit_expr(cond)?;
                self.push_op(Op::JmpZ);
                let patch_else = self.code.len();
                self.push_i16(0);
                self.emit_expr(then_e)?;
                self.push_op(Op::Jmp);
                let patch_end = self.code.len();
                self.push_i16(0);
                self.patch_jump_to_here(patch_else);
                self.emit_expr(else_e)?;
                self.patch_jump_to_here(patch_end);
            }
            Expr::BinOp(l, op, r) => {
                if matches!(op, BinOp::And | BinOp::Or) {
                    return self.emit_short_circuit(*op, l, r);
                }
                let lt = self.infer_expr_type(l)?;
                let rt = self.infer_expr_type(r)?;
                let out_t = self.infer_binop_type(*op, lt, rt)?;

                match op {
                    BinOp::Mul if out_t == ValueType::Color && lt == ValueType::Scalar && rt == ValueType::Color => {
                        self.emit_expr(r)?;
                        self.emit_expr(l)?;
                        self.push_op(Op::MulCF);
                    }
                    BinOp::Mul if out_t == ValueType::Color && lt == ValueType::Color && rt == ValueType::Scalar => {
                        self.emit_expr(l)?;
                        self.emit_expr(r)?;
                        self.push_op(Op::MulCF);
                    }
                    _ => {
                        self.emit_expr(l)?;
                        self.emit_expr(r)?;
                        match op {
                            BinOp::Add if out_t == ValueType::Color => self.push_op(Op::AddC),
                            BinOp::Sub if out_t == ValueType::Color => self.push_op(Op::SubC),
                            BinOp::Mul if out_t == ValueType::Color => self.push_op(Op::MulC),
                            BinOp::Div if out_t == ValueType::Color => self.push_op(Op::DivC),
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
                }
            }
            Expr::Call(name, args) => {
                let load_hidden = |this: &mut Self, slot: u8| {
                    this.push_op(Op::Load);
                    this.push_u8(slot);
                };

                match name.as_str() {
                    "vec2" => {
                        if args.len() != 2 {
                            return Err("vec2 requires exactly 2 scalar arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Vec2);
                        return Ok(());
                    }
                    "vec3" => {
                        if args.len() != 3 {
                            return Err("vec3 requires exactly 3 scalar arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Vec3);
                        return Ok(());
                    }
                    "vec4" => {
                        if args.len() != 4 {
                            return Err("vec4 requires exactly 4 scalar arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Vec4);
                        return Ok(());
                    }
                    "matrix2" => {
                        if args.len() != 4 {
                            return Err("matrix2 requires exactly 4 scalar arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Mat2);
                        return Ok(());
                    }
                    "matrix3" => {
                        if args.len() != 9 {
                            return Err("matrix3 requires exactly 9 scalar arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Mat3);
                        return Ok(());
                    }
                    "matrix4" => {
                        if args.len() != 16 {
                            return Err("matrix4 requires exactly 16 scalar arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Mat4);
                        return Ok(());
                    }
                    "dot" => {
                        if args.len() != 2 {
                            return Err("dot requires 2 vector arguments".to_string());
                        }
                        self.emit_expr(&args[0])?;
                        self.emit_expr(&args[1])?;
                        self.push_op(Op::Dot);
                        return Ok(());
                    }
                    "cross" => {
                        if args.len() != 2 {
                            return Err("cross requires 2 vec3 arguments".to_string());
                        }
                        self.emit_expr(&args[0])?;
                        self.emit_expr(&args[1])?;
                        self.push_op(Op::Cross);
                        return Ok(());
                    }
                    "normalize" => {
                        if args.len() != 1 {
                            return Err("normalize requires 1 vector argument".to_string());
                        }
                        self.emit_expr(&args[0])?;
                        self.push_op(Op::Normalize);
                        return Ok(());
                    }
                    "reflect" => {
                        if args.len() != 2 {
                            return Err("reflect requires 2 vector arguments".to_string());
                        }
                        self.emit_expr(&args[0])?;
                        self.emit_expr(&args[1])?;
                        self.push_op(Op::Reflect);
                        return Ok(());
                    }
                    "refract" => {
                        if args.len() != 3 {
                            return Err("refract requires 3 arguments".to_string());
                        }
                        for a in args {
                            self.emit_expr(a)?;
                        }
                        self.push_op(Op::Refract);
                        return Ok(());
                    }
                    _ => {}
                }

                    if let Some(param_types) = self.fn_param_types.get(name) {
                        if param_types.len() != args.len() {
                            return Err(format!("call to {} expects {} args", name, param_types.len()));
                        }
                        for (idx, (arg, expected)) in args.iter().zip(param_types.iter()).enumerate() {
                            let actual = self.infer_expr_type(arg)?;
                            if actual != *expected && actual != ValueType::Unknown && *expected != ValueType::Unknown {
                                return Err(format!(
                                    "arg {} of {} has type {}, expected {}",
                                    idx + 1,
                                    name,
                                    value_type_name(&actual),
                                    value_type_name(expected)
                                ));
                            }
                        }
                    }

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
                        let mut writebacks: Vec<(u8, u8)> = Vec::new();
                        if let Some(param_quals) = self.fn_param_quals.get(name) {
                            for (arg_idx, qual_flags) in param_quals.iter().enumerate() {
                                if (qual_flags & (QUAL_OUT | QUAL_INOUT)) == 0 {
                                    continue;
                                }
                                let Expr::Var(var_name) = &args[arg_idx] else {
                                    return Err(format!(
                                        "arg {} of {} is out/inout and must be a variable",
                                        arg_idx + 1,
                                        name
                                    ));
                                };
                                if self.readonly_locals.contains(var_name) {
                                    return Err(format!(
                                        "arg {} of {} is out/inout but variable {} is read-only",
                                        arg_idx + 1,
                                        name,
                                        var_name
                                    ));
                                }
                                let Some(&caller_slot) = self.locals.get(var_name.as_str()) else {
                                    if self.extern_slots.contains_key(var_name.as_str()) {
                                        return Err(format!(
                                            "arg {} of {} is out/inout and cannot bind to extern {}",
                                            arg_idx + 1,
                                            name,
                                            var_name
                                        ));
                                    }
                                    return Err(format!(
                                        "arg {} of {} is out/inout and must bind to a local variable",
                                        arg_idx + 1,
                                        name
                                    ));
                                };
                                writebacks.push((arg_idx as u8, caller_slot));
                            }
                        }

                        if writebacks.is_empty() {
                            self.push_op(Op::Call);
                            self.push_u8(idx);
                            self.push_u8(args.len() as u8);
                        } else {
                            self.push_op(Op::CallExt);
                            self.push_u8(idx);
                            self.push_u8(args.len() as u8);
                            self.push_u8(writebacks.len() as u8);
                            for (callee_idx, caller_slot) in writebacks {
                                self.push_u8(callee_idx);
                                self.push_u8(caller_slot);
                            }
                        }
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
                if self.readonly_locals.contains(name) {
                    return Err(format!("cannot assign to read-only parameter {}", name));
                }
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
            Stmt::AssignIndex(name, idx, e) => {
                if self.readonly_locals.contains(name) {
                    return Err(format!("cannot assign to read-only parameter {}", name));
                }
                let Some(&slot) = self.locals.get(name.as_str()) else {
                    return Err(format!("undef var {name}"));
                };
                let base_t = *self.local_types.get(name).unwrap_or(&ValueType::Unknown);
                let Some(elem_t) = array_elem_type(base_t) else {
                    return Err(format!("{} is not an array", name));
                };
                let idx_t = self.infer_expr_type(idx)?;
                if idx_t != ValueType::Scalar {
                    return Err("array index must be scalar".to_string());
                }
                let rhs_t = self.infer_expr_type(e)?;
                if elem_t != ValueType::Unknown && rhs_t != ValueType::Unknown && elem_t != rhs_t {
                    return Err(format!(
                        "type mismatch in indexed assignment to {}: expected {}, got {}",
                        name,
                        value_type_name(&elem_t),
                        value_type_name(&rhs_t)
                    ));
                }
                self.push_op(Op::Load);
                self.push_u8(slot);
                self.emit_expr(idx)?;
                self.emit_expr(e)?;
                self.push_op(Op::ArraySet);
            }
            Stmt::ReturnVals(vals) => {
                self.emit_return_vals(vals)?;
            }
            Stmt::If(cond, then_b, else_b) => {
                if self.infer_expr_type(cond)? != ValueType::Bool {
                    return Err("if condition must be bool".to_string());
                }
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
                if self.infer_expr_type(cond)? != ValueType::Bool {
                    return Err("while condition must be bool".to_string());
                }
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
        Expr::Num(_) | Expr::Bool(_) | Expr::Var(_) => e.clone(),
        Expr::ArrayLit(items) => Expr::ArrayLit(items.iter().map(fold_expr).collect()),
        Expr::Index(base, idx) => Expr::Index(Box::new(fold_expr(base)), Box::new(fold_expr(idx))),
        Expr::Field(inner, f) => Expr::Field(Box::new(fold_expr(inner)), f.clone()),
        Expr::Ternary(cond, then_e, else_e) => {
            let fc = fold_expr(cond);
            if let Expr::Num(n) = fc {
                if n != 0.0 {
                    return fold_expr(then_e);
                }
                return fold_expr(else_e);
            }
            Expr::Ternary(
                Box::new(fc),
                Box::new(fold_expr(then_e)),
                Box::new(fold_expr(else_e)),
            )
        }
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

fn const_hue2rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn const_hsl_to_rgb(h: f32, s: f32, l: f32) -> [f32; 3] {
    if s == 0.0 {
        return [l, l, l];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    [
        const_hue2rgb(p, q, h + 1.0 / 3.0),
        const_hue2rgb(p, q, h),
        const_hue2rgb(p, q, h - 1.0 / 3.0),
    ]
}

fn const_hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    if s == 0.0 {
        return [v, v, v];
    }
    let i = (h * 6.0) as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match i % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

fn eval_const_number(e: &Expr) -> Option<f32> {
    match fold_expr(e) {
        Expr::Num(n) => Some(n),
        _ => None,
    }
}

fn eval_const_color(e: &Expr) -> Option<[f32; 3]> {
    match e {
        Expr::Call(name, args) => match name.as_str() {
            "rgb" if args.len() == 3 => {
                let r = eval_const_number(&args[0])? / 255.0;
                let g = eval_const_number(&args[1])? / 255.0;
                let b = eval_const_number(&args[2])? / 255.0;
                Some([r, g, b])
            }
            "rgba" if args.len() == 4 => {
                let r = eval_const_number(&args[0])? / 255.0;
                let g = eval_const_number(&args[1])? / 255.0;
                let b = eval_const_number(&args[2])? / 255.0;
                let mut a = eval_const_number(&args[3])?;
                if a > 1.0 {
                    a /= 255.0;
                }
                a = a.clamp(0.0, 1.0);
                Some([r * a, g * a, b * a])
            }
            "hsl" if args.len() == 3 => Some(const_hsl_to_rgb(
                eval_const_number(&args[0])?,
                eval_const_number(&args[1])?,
                eval_const_number(&args[2])?,
            )),
            "hsv" if args.len() == 3 => Some(const_hsv_to_rgb(
                eval_const_number(&args[0])?,
                eval_const_number(&args[1])?,
                eval_const_number(&args[2])?,
            )),
            "gray" if args.len() == 1 => {
                let v = eval_const_number(&args[0])?;
                Some([v, v, v])
            }
            "mix" | "mixc" if args.len() == 3 => {
                let a = eval_const_color(&args[0])?;
                let b = eval_const_color(&args[1])?;
                let t = eval_const_number(&args[2])?;
                Some([
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ])
            }
            _ => None,
        },
        _ => None,
    }
}

fn expr_is_pure(e: &Expr) -> bool {
    match e {
        Expr::Num(_) | Expr::Bool(_) | Expr::Var(_) => true,
        Expr::ArrayLit(items) => items.iter().all(expr_is_pure),
        Expr::BinOp(a, _, b) => expr_is_pure(a) && expr_is_pure(b),
        Expr::UnOp(_, a) => expr_is_pure(a),
        Expr::Ternary(c, t, f) => expr_is_pure(c) && expr_is_pure(t) && expr_is_pure(f),
        Expr::Field(a, _) => expr_is_pure(a),
        Expr::Index(a, b) => expr_is_pure(a) && expr_is_pure(b),
        Expr::Call(_, _) => false,
    }
}

fn expr_key(e: &Expr) -> Option<String> {
    match e {
        Expr::Num(v) => Some(format!("n:{v:?}")),
        Expr::Bool(v) => Some(format!("b:{v}")),
        Expr::Var(v) => Some(format!("v:{v}")),
        Expr::ArrayLit(items) => {
            let mut out = String::from("arr[");
            for item in items {
                out.push_str(&expr_key(item)?);
                out.push(';');
            }
            out.push(']');
            Some(out)
        }
        Expr::BinOp(a, op, b) => Some(format!("bin:{op:?}({},{})", expr_key(a)?, expr_key(b)?)),
        Expr::UnOp(op, a) => Some(format!("un:{op:?}({})", expr_key(a)?)),
        Expr::Ternary(c, t, f) => Some(format!("ter:({})?({}):({})", expr_key(c)?, expr_key(t)?, expr_key(f)?)),
        Expr::Call(_, _) => None,
        Expr::Field(a, f) => Some(format!("field:{f}({})", expr_key(a)?)),
        Expr::Index(a, b) => Some(format!("idx:({})[{}]", expr_key(a)?, expr_key(b)?)),
    }
}

fn collect_expr_reads(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::Var(v) => {
            out.insert(v.clone());
        }
        Expr::ArrayLit(items) => {
            for item in items {
                collect_expr_reads(item, out);
            }
        }
        Expr::BinOp(a, _, b) | Expr::Index(a, b) => {
            collect_expr_reads(a, out);
            collect_expr_reads(b, out);
        }
        Expr::UnOp(_, a) | Expr::Field(a, _) => collect_expr_reads(a, out),
        Expr::Ternary(c, t, f) => {
            collect_expr_reads(c, out);
            collect_expr_reads(t, out);
            collect_expr_reads(f, out);
        }
        Expr::Call(_, args) => {
            for arg in args {
                collect_expr_reads(arg, out);
            }
        }
        Expr::Num(_) | Expr::Bool(_) => {}
    }
}

fn collect_stmt_reads(s: &Stmt, out: &mut HashSet<String>) {
    match s {
        Stmt::Let(_, e) | Stmt::Assign(_, e) | Stmt::Expr(e) => collect_expr_reads(e, out),
        Stmt::AssignIndex(name, idx, e) => {
            out.insert(name.clone());
            collect_expr_reads(idx, out);
            collect_expr_reads(e, out);
        }
        Stmt::ReturnVals(vals) => {
            for v in vals {
                collect_expr_reads(v, out);
            }
        }
        Stmt::If(cond, then_b, else_b) => {
            collect_expr_reads(cond, out);
            for stmt in then_b {
                collect_stmt_reads(stmt, out);
            }
            for stmt in else_b {
                collect_stmt_reads(stmt, out);
            }
        }
        Stmt::While(cond, body) => {
            collect_expr_reads(cond, out);
            for stmt in body {
                collect_stmt_reads(stmt, out);
            }
        }
        Stmt::For(_, lo, hi, body) => {
            collect_expr_reads(lo, out);
            collect_expr_reads(hi, out);
            for stmt in body {
                collect_stmt_reads(stmt, out);
            }
        }
        Stmt::Break | Stmt::Continue => {}
    }
}

fn optimize_stmts(stmts: &[Stmt]) -> Vec<Stmt> {
    optimize_stmts_inner(stmts, false)
}

fn optimize_stmts_inner(stmts: &[Stmt], in_loop_body: bool) -> Vec<Stmt> {
    let mut folded = Vec::new();
    let mut terminated = false;
    let mut expr_cache: HashMap<String, String> = HashMap::new();

    for s in stmts {
        if terminated {
            break;
        }

        let cse_replace = |e: Expr| -> Expr {
            if !expr_is_pure(&e) {
                return e;
            }
            let Some(k) = expr_key(&e) else {
                return e;
            };
            if let Some(prev) = expr_cache.get(&k) {
                return Expr::Var(prev.clone());
            }
            e
        };

        let ns = match s {
            Stmt::Let(n, e) => {
                let mut fe = fold_expr(e);
                fe = cse_replace(fe);
                if expr_is_pure(&fe)
                    && let Some(k) = expr_key(&fe)
                {
                    expr_cache.insert(k, n.clone());
                } else {
                    expr_cache.clear();
                }
                Stmt::Let(n.clone(), fe)
            }
            Stmt::Assign(n, e) => {
                let mut fe = fold_expr(e);
                fe = cse_replace(fe);
                expr_cache.clear();
                if expr_is_pure(&fe)
                    && let Some(k) = expr_key(&fe)
                {
                    expr_cache.insert(k, n.clone());
                }
                Stmt::Assign(n.clone(), fe)
            }
            Stmt::AssignIndex(n, idx, e) => {
                expr_cache.clear();
                Stmt::AssignIndex(n.clone(), fold_expr(idx), fold_expr(e))
            }
            Stmt::ReturnVals(vals) => {
                terminated = true;
                expr_cache.clear();
                Stmt::ReturnVals(vals.iter().map(fold_expr).collect())
            }
            Stmt::If(cond, then_b, else_b) => {
                expr_cache.clear();
                let c = fold_expr(cond);
                if let Expr::Num(n) = c {
                    if n != 0.0 {
                        folded.extend(optimize_stmts_inner(then_b, in_loop_body));
                        continue;
                    }
                    folded.extend(optimize_stmts_inner(else_b, in_loop_body));
                    continue;
                }
                Stmt::If(
                    c,
                    optimize_stmts_inner(then_b, in_loop_body),
                    optimize_stmts_inner(else_b, in_loop_body),
                )
            }
            Stmt::While(cond, body) => {
                expr_cache.clear();
                let c = fold_expr(cond);
                if let Expr::Num(n) = c
                    && n == 0.0
                {
                    continue;
                }
                Stmt::While(c, optimize_stmts_inner(body, true))
            }
            Stmt::For(v, lo, hi, body) => {
                expr_cache.clear();
                Stmt::For(
                    v.clone(),
                    fold_expr(lo),
                    fold_expr(hi),
                    optimize_stmts_inner(body, true),
                )
            }
            Stmt::Break => {
                terminated = true;
                expr_cache.clear();
                Stmt::Break
            }
            Stmt::Continue => {
                terminated = true;
                expr_cache.clear();
                Stmt::Continue
            }
            Stmt::Expr(e) => {
                let fe = fold_expr(e);
                if !expr_is_pure(&fe) {
                    expr_cache.clear();
                }
                Stmt::Expr(fe)
            }
        };
        folded.push(ns);
    }

    let mut live: HashSet<String> = HashSet::new();
    let mut out_rev = Vec::new();
    for s in folded.into_iter().rev() {
        match &s {
            Stmt::Let(n, e) | Stmt::Assign(n, e) => {
                if !in_loop_body && !live.contains(n) && expr_is_pure(e) {
                    continue;
                }
                live.remove(n);
                collect_expr_reads(e, &mut live);
            }
            Stmt::AssignIndex(n, idx, e) => {
                live.insert(n.clone());
                collect_expr_reads(idx, &mut live);
                collect_expr_reads(e, &mut live);
            }
            Stmt::ReturnVals(vals) => {
                for v in vals {
                    collect_expr_reads(v, &mut live);
                }
            }
            Stmt::If(cond, then_b, else_b) => {
                collect_expr_reads(cond, &mut live);
                for stmt in then_b {
                    collect_stmt_reads(stmt, &mut live);
                }
                for stmt in else_b {
                    collect_stmt_reads(stmt, &mut live);
                }
            }
            Stmt::While(cond, body) => {
                collect_expr_reads(cond, &mut live);
                for stmt in body {
                    collect_stmt_reads(stmt, &mut live);
                }
            }
            Stmt::For(v, lo, hi, body) => {
                live.remove(v);
                collect_expr_reads(lo, &mut live);
                collect_expr_reads(hi, &mut live);
                for stmt in body {
                    collect_stmt_reads(stmt, &mut live);
                }
            }
            Stmt::Expr(e) => collect_expr_reads(e, &mut live),
            Stmt::Break | Stmt::Continue => {}
        }
        out_rev.push(s);
    }

    out_rev.reverse();
    out_rev
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

const QUAL_OUT: u8 = 1 << 0;
const QUAL_INOUT: u8 = 1 << 1;

fn param_qual_flags(quals: &[ParamQualifier]) -> u8 {
    let mut out = 0u8;
    for q in quals {
        match q {
            ParamQualifier::Out => out |= QUAL_OUT,
            ParamQualifier::InOut => out |= QUAL_INOUT,
            ParamQualifier::In | ParamQualifier::Const => {}
        }
    }
    out
}

fn param_to_valuetype(ty: &TypeSpec) -> ValueType {
    match ty {
        TypeSpec::Color => ValueType::Color,
        TypeSpec::Float | TypeSpec::Int | TypeSpec::Char => ValueType::Scalar,
        TypeSpec::Bool => ValueType::Bool,
        TypeSpec::Vec(n) => ValueType::Vector(*n),
        TypeSpec::Mat(n) => ValueType::Matrix(*n),
        TypeSpec::Array(inner, _) => match param_to_valuetype(inner) {
            ValueType::Scalar => ValueType::ArrayScalar,
            ValueType::Color => ValueType::ArrayColor,
            ValueType::Bool => ValueType::ArrayBool,
            ValueType::Vector(n) => ValueType::ArrayVector(n),
            ValueType::Matrix(n) => ValueType::ArrayMatrix(n),
            _ => ValueType::ArrayUnknown,
        },
    }
}

fn is_scalar_like(ty: ValueType) -> bool {
    matches!(ty, ValueType::Scalar)
}

fn array_elem_type(ty: ValueType) -> Option<ValueType> {
    match ty {
        ValueType::ArrayScalar => Some(ValueType::Scalar),
        ValueType::ArrayColor => Some(ValueType::Color),
        ValueType::ArrayBool => Some(ValueType::Bool),
        ValueType::ArrayVector(n) => Some(ValueType::Vector(n)),
        ValueType::ArrayMatrix(n) => Some(ValueType::Matrix(n)),
        ValueType::ArrayUnknown => Some(ValueType::Unknown),
        _ => None,
    }
}

fn vector_len(ty: ValueType) -> Option<usize> {
    match ty {
        ValueType::Color => Some(3),
        ValueType::Vector(n) => Some(n),
        _ => None,
    }
}

fn swizzle_result_type(inner: ValueType, field: &str) -> CompileResult<ValueType> {
    if field.is_empty() || field.len() > 4 {
        return Err("swizzle must have 1 to 4 components".to_string());
    }
    let valid = match inner {
        ValueType::Color => "rgba",
        ValueType::Vector(2) => "xy",
        ValueType::Vector(3) => "xyz",
        ValueType::Vector(4) => "xyzw",
        ValueType::Unknown => return Ok(ValueType::Unknown),
        _ => return Err("swizzle requires a vector or color value".to_string()),
    };
    if !field.chars().all(|ch| valid.contains(ch)) {
        return Err(format!("invalid swizzle .{} for {}", field, value_type_name(&inner)));
    }
    Ok(match field.len() {
        1 => ValueType::Scalar,
        n => ValueType::Vector(n),
    })
}

/// Compile parsed functions to bytecode.
pub fn compile_fns(fns: &[FnDef]) -> CompileResult<CompiledShader> {
    compile_fns_with_externs(fns, &[])
}

pub fn compile_fns_with_externs(
    fns: &[FnDef],
    externs: &[ExternDecl],
) -> CompileResult<CompiledShader> {
    let optimized = optimize_fns(fns);
    let fn_names: Vec<String> = optimized.iter().map(|f| f.name.clone()).collect();
    let fn_param_types: HashMap<String, Vec<ValueType>> = optimized
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                f.params.iter().map(|p| param_to_valuetype(&p.ty)).collect(),
            )
        })
        .collect();
    let fn_param_quals: HashMap<String, Vec<u8>> = optimized
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                f.params
                    .iter()
                    .map(|p| param_qual_flags(&p.qual))
                    .collect(),
            )
        })
        .collect();
    let fn_ret_types: HashMap<String, ValueType> = optimized
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                match f.ret_type {
                    ReturnType::Color => ValueType::Color,
                    ReturnType::Char | ReturnType::CharFg | ReturnType::FgBg | ReturnType::CharFgBg => ValueType::Scalar,
                },
            )
        })
        .collect();
    let mut compiled = Vec::new();

    let mut extern_slots = HashMap::new();
    let mut extern_types = HashMap::new();
    let mut next_slot: u8 = 11;
    for ext in externs {
        if extern_slots.contains_key(ext.name.as_str()) {
            return Err(format!("duplicate extern declaration: {}", ext.name));
        }
        if next_slot == u8::MAX {
            return Err("too many externs (slot overflow)".to_string());
        }
        extern_slots.insert(ext.name.clone(), next_slot);
        let vt = match ext.ty {
            ExternType::Color | ExternType::CharFg | ExternType::FgBg | ExternType::CharFgBg => {
                ValueType::Color
            }
            ExternType::Number | ExternType::Char => ValueType::Scalar,
            ExternType::Bool => ValueType::Bool,
        };
        extern_types.insert(ext.name.clone(), vt);
        next_slot = next_slot.saturating_add(1);
    }

    for f in &optimized {
        let mut em = Emitter {
            code: Vec::new(),
            consts: Vec::new(),
            locals: HashMap::new(),
            local_types: HashMap::new(),
            readonly_locals: HashSet::new(),
            local_count: 0,
            fn_names: &fn_names,
            fn_param_types: &fn_param_types,
            fn_param_quals: &fn_param_quals,
            fn_ret_types: &fn_ret_types,
            extern_slots: &extern_slots,
            extern_types: &extern_types,
            ret_type: f.ret_type,
            loops: Vec::new(),
        };

        for p in &f.params {
            em.local(&p.name);
            em.local_types.insert(p.name.clone(), param_to_valuetype(&p.ty));
            if p.qual.contains(&ParamQualifier::Const) || p.qual.contains(&ParamQualifier::In) {
                em.readonly_locals.insert(p.name.clone());
            }
        }

        let hidden_plus_externs = 11u8.saturating_add(externs.len() as u8);
        if em.local_count < hidden_plus_externs {
            em.local_count = hidden_plus_externs;
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
        externs: externs.to_vec(),
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
    let program = parse_program(&expanded)?;
    compile_fns_with_externs(&program.fns, &program.externs)
}
