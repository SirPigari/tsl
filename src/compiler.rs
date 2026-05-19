use std::collections::HashMap;

type CompileResult<T> = Result<T, String>;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Num(f32),
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
    Return(Expr),
    Return2(Expr, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Expr, Vec<Stmt>),
    Expr(Expr),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReturnType {
    Color,
    Char,
    ColorChar,
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
    fn expect_ident(&mut self) -> CompileResult<String> {
        if let Token::Ident(s) = self.next() {
            Ok(s)
        } else {
            Err("expected ident".to_string())
        }
    }

    fn parse_fn(&mut self) -> CompileResult<FnDef> {
        self.eat(&Token::Fn);
        let name = self.expect_ident()?;
        self.eat(&Token::LParen);
        let mut params = Vec::new();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            params.push(self.expect_ident()?);
            self.eat(&Token::Comma);
        }
        self.eat(&Token::RParen);
        let ret_type = if self.eat(&Token::Arrow) {
            if self.eat(&Token::LParen) {
                self.expect_ident()?;
                self.eat(&Token::Comma);
                self.expect_ident()?;
                self.eat(&Token::RParen);
                ReturnType::ColorChar
            } else {
                match self.expect_ident()?.as_str() {
                    "char" => ReturnType::Char,
                    _ => ReturnType::Color,
                }
            }
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
        self.eat(&Token::End);
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> CompileResult<Stmt> {
        let stmt = match self.peek().clone() {
            Token::Let => {
                self.next();
                let name = self.expect_ident()?;
                self.eat(&Token::Eq);
                let e = self.parse_expr();
                self.eat(&Token::Semicolon);
                Stmt::Let(name, e)
            }
            Token::Return => {
                self.next();
                if matches!(self.peek(), Token::LParen) {
                    self.next();
                    let a = self.parse_expr();
                    if self.eat(&Token::Comma) {
                        let b = self.parse_expr();
                        self.eat(&Token::RParen);
                        self.eat(&Token::Semicolon);
                        Stmt::Return2(a, b)
                    } else {
                        self.eat(&Token::RParen);
                        self.eat(&Token::Semicolon);
                        Stmt::Return(a)
                    }
                } else {
                    let e = self.parse_expr();
                    self.eat(&Token::Semicolon);
                    Stmt::Return(e)
                }
            }
            Token::If => {
                self.next();
                let cond = self.parse_expr();
                self.eat(&Token::Do);
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
                self.eat(&Token::End);
                Stmt::If(cond, then_b, else_b)
            }
            Token::While => {
                self.next();
                let cond = self.parse_expr();
                self.eat(&Token::Do);
                let body = self.parse_block_end()?;
                Stmt::While(cond, body)
            }
            Token::For => {
                self.next();
                let var = self.expect_ident()?;
                self.eat(&Token::In);
                let lo = self.parse_expr();
                self.eat(&Token::DotDot);
                let hi = self.parse_expr();
                self.eat(&Token::Do);
                let body = self.parse_block_end()?;
                Stmt::For(var, lo, hi, body)
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
                        self.eat(&Token::Comma);
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
    PushF = 0, // [f32 x4] push float
    PushC = 1, // [f32 r, f32 g, f32 b] push color
    Pop = 2,
    // locals
    Load = 3,  // [u8 slot]
    Store = 4, // [u8 slot]
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
    MulCF = 24, // color * float
    // cmp -> push 0.0 or 1.0
    EqF = 30,
    NeF = 31,
    LtF = 32,
    GtF = 33,
    LeF = 34,
    GeF = 35,
    AndF = 36,
    OrF = 37,
    NotF = 38,
    // math builtins (f32 -> f32)
    Sin = 40,
    Cos = 41,
    Tan = 42,
    Asin = 43,
    Acos = 44,
    Atan = 45,
    Atan2 = 46, // two args
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
    Clamp = 58, // 3 args
    Mix = 59,   // lerp(a, b, t) 3 args
    Step = 60,
    Smoothstep = 61,
    Sign = 62,
    Length2 = 63,
    // color builtins
    Rgb = 70,   // (r,g,b)
    Rgba = 71,  // (r,g,b,a) - alpha ignored for now
    Hsl = 72,   // (h,s,l)
    Hsv = 73,   // (h,s,v)
    Gray = 74,  // (v)
    Mix2C = 75, // mix(colorA, colorB, t)
    // field access
    GetR = 80,
    GetG = 81,
    GetB = 82,
    // char classify (consume char_code float, push 0.0 or 1.0)
    IsSpace = 83,
    IsDigit = 84,
    IsAlpha = 85,
    IsUpper = 86,
    IsLower = 87,
    // control
    Jmp = 90,  // [i16 offset]
    JmpZ = 91, // [i16 offset] jump if top==0
    Ret = 99,
    // call user fn
    Call = 100, // [u8 fn_idx, u8 argc]
    // rng
    Rand = 101,        // push random f32 in [0.0, 1.0)
    RandBetween = 102, // pop b, pop a, push random f32 in [a, b)
    MakeGlyph = 103,   // pop two vals (color+char in any order), push G(color, char)
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
    pub entry: usize,          // index of main fn
    pub entry_ret: ReturnType, // return type of main
}

const MAGIC: &[u8; 4] = b"CTSL";
const VERSION: u8 = 3;

impl CompiledShader {
    /// Serialise compiled shader to `.ctsl` bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.push(self.fns.len() as u8);
        out.push(self.entry as u8);
        out.push(if self.entry_ret == ReturnType::Char {
            1
        } else if self.entry_ret == ReturnType::ColorChar {
            2
        } else {
            0
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
        if data.len() < 7 || &data[..4] != MAGIC {
            return Err("bad magic");
        }
        let ver = data[4];
        if ver != 1 && ver != 2 && ver != VERSION {
            return Err("version mismatch");
        }
        let fn_count = data[5] as usize;
        let entry = data[6] as usize;
        let (entry_ret, mut pos) = if ver >= 2 {
            let r = match data.get(7).copied() {
                Some(1) => ReturnType::Char,
                Some(2) => ReturnType::ColorChar,
                _ => ReturnType::Color,
            };
            (r, 8usize)
        } else {
            (ReturnType::Color, 7usize)
        };
        let mut fns = Vec::with_capacity(fn_count);
        for _ in 0..fn_count {
            let nlen = data[pos] as usize;
            pos += 1;
            let name = std::str::from_utf8(&data[pos..pos + nlen])
                .map_err(|_| "utf8")?
                .to_string();
            pos += nlen;
            let params = data[pos];
            pos += 1;
            let clen = u32::from_le_bytes(data[pos..pos + 4].try_into().map_err(|_| "truncated")?)
                as usize;
            pos += 4;
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

struct Emitter<'a> {
    code: Vec<u8>,
    consts: Vec<f32>,
    locals: HashMap<String, u8>,
    local_count: u8,
    fn_names: &'a [String],
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

    #[allow(unused)]
    fn const_f(&mut self, v: f32) -> usize {
        for (i, &c) in self.consts.iter().enumerate() {
            if c == v {
                return i;
            }
        }
        self.consts.push(v);
        self.consts.len() - 1
    }

    fn local(&mut self, name: &str) -> u8 {
        if let Some(&s) = self.locals.get(name) {
            return s;
        }
        let s = self.local_count;
        self.locals.insert(name.to_string(), s);
        self.local_count += 1;
        s
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
                if name == "original" {
                    self.push_op(Op::Load);
                    self.push_u8(8);
                    return Ok(());
                }
                if name == "time" {
                    self.push_op(Op::Load);
                    self.push_u8(9);
                    return Ok(());
                }
                if name == "char_code" {
                    self.push_op(Op::Load);
                    self.push_u8(7);
                    return Ok(());
                }
                if name == "rand" {
                    match args.len() {
                        0 => {
                            self.push_op(Op::Rand);
                        }
                        2 => {
                            self.emit_expr(&args[0])?;
                            self.emit_expr(&args[1])?;
                            self.push_op(Op::RandBetween);
                        }
                        _ => return Err("rand takes 0 or 2 args".to_string()),
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
                            self.push_op(Op::Mix);
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

    fn emit_stmt(&mut self, s: &Stmt) -> CompileResult<()> {
        match s {
            Stmt::Let(name, e) | Stmt::Assign(name, e) => {
                self.emit_expr(e)?;
                let slot = self.local(name);
                self.push_op(Op::Store);
                self.push_u8(slot);
            }
            Stmt::Return(e) => {
                self.emit_expr(e)?;
                self.push_op(Op::Ret);
            }
            Stmt::Return2(a, b) => {
                self.emit_expr(a)?;
                self.emit_expr(b)?;
                self.push_op(Op::MakeGlyph);
                self.push_op(Op::Ret);
            }
            Stmt::If(cond, then_b, else_b) => {
                self.emit_expr(cond)?;
                // JmpZ to else
                self.push_op(Op::JmpZ);
                let patch_else = self.code.len();
                self.push_i16(0);
                self.emit_stmts(then_b)?;
                if !else_b.is_empty() {
                    // Jmp over else
                    self.push_op(Op::Jmp);
                    let patch_end = self.code.len();
                    self.push_i16(0);
                    let else_start = self.code.len() as i16 - (patch_else as i16 + 2);
                    let bytes = else_start.to_le_bytes();
                    self.code[patch_else] = bytes[0];
                    self.code[patch_else + 1] = bytes[1];
                    self.emit_stmts(else_b)?;
                    let end = self.code.len() as i16 - (patch_end as i16 + 2);
                    let bytes = end.to_le_bytes();
                    self.code[patch_end] = bytes[0];
                    self.code[patch_end + 1] = bytes[1];
                } else {
                    let off = self.code.len() as i16 - (patch_else as i16 + 2);
                    let bytes = off.to_le_bytes();
                    self.code[patch_else] = bytes[0];
                    self.code[patch_else + 1] = bytes[1];
                }
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len() as i16;
                self.emit_expr(cond)?;
                self.push_op(Op::JmpZ);
                let patch = self.code.len();
                self.push_i16(0);
                self.emit_stmts(body)?;
                // jump back
                self.push_op(Op::Jmp);
                let back = loop_start - (self.code.len() as i16 + 2);
                self.push_i16(back);
                let fwd = self.code.len() as i16 - (patch as i16 + 2);
                let bytes = fwd.to_le_bytes();
                self.code[patch] = bytes[0];
                self.code[patch + 1] = bytes[1];
            }
            Stmt::For(var, lo, hi, body) => {
                self.emit_expr(lo)?;
                let slot = self.local(var);
                self.push_op(Op::Store);
                self.push_u8(slot);
                // check
                let loop_start = self.code.len() as i16;
                self.push_op(Op::Load);
                self.push_u8(slot);
                self.emit_expr(hi)?;
                self.push_op(Op::LtF);
                self.push_op(Op::JmpZ);
                let patch = self.code.len();
                self.push_i16(0);
                self.emit_stmts(body)?;
                // i = i + 1
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
                let fwd = self.code.len() as i16 - (patch as i16 + 2);
                let bytes = fwd.to_le_bytes();
                self.code[patch] = bytes[0];
                self.code[patch + 1] = bytes[1];
            }
            Stmt::Expr(e) => {
                self.emit_expr(e)?;
                self.push_op(Op::Pop);
            }
        }
        Ok(())
    }
}

/// Compile parsed functions to bytecode.
pub fn compile_fns(fns: &[FnDef]) -> CompileResult<CompiledShader> {
    let fn_names: Vec<String> = fns.iter().map(|f| f.name.clone()).collect();
    let mut compiled = Vec::new();
    for f in fns {
        let mut em = Emitter {
            code: Vec::new(),
            consts: Vec::new(),
            locals: HashMap::new(),
            local_count: 0,
            fn_names: &fn_names,
        };
        // params are first locals (slots 0..params.len())
        // Slots 0..10 are reserved for renderer-injected values; user `let`
        // variables must start at slot 10 to avoid clobbering hidden slots
        // (7=original, 8=time, 9=char_code etc.).
        for p in &f.params {
            em.local(p);
        }
        // Bump local_count up to the reserved boundary so user vars start at 10.
        if em.local_count < 10 {
            em.local_count = 10;
        }
        em.emit_stmts(&f.body)?;
        // implicit return 0
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
    let entry_ret = fns
        .get(entry)
        .map(|f| f.ret_type)
        .unwrap_or(ReturnType::Color);
    Ok(CompiledShader {
        fns: compiled,
        entry,
        entry_ret,
    })
}

/// Compile TSL source to a `CompiledShader`.
pub fn compile(src: &str) -> Result<CompiledShader, String> {
    let fns = parse(src)?;
    compile_fns(&fns)
}
