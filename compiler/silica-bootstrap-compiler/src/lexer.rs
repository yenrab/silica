/*
   Copyright 2026 Lee Scott Barney

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use crate::errors::{Result, SourceLocation, lexer_error, lexer_error_with_metadata};

/// Maximum line number considered sane; used for position-tracking diagnostics.
const MAX_SANE_LINE: usize = 10_000_000;

/// Token represents a lexical token in Silica source code
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub location: SourceLocation,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, location: SourceLocation) -> Self {
        Token {
            kind,
            lexeme,
            location,
        }
    }
}

/// TokenKind enumerates all possible token types in Silica
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (33 total)
    Actor, ActorRef, As, Atomic, Bool, Buf, Case, Cast, Char, Concurrency, CoreId, CoreSet, DeviceIO, EfficiencyCores, Includes, PerformanceCores,
    Do, Effect, Else, End, Enum, Export, False, Float16, Float32, Float64, Fn, For, From, If,
    Impl, Import, Int8, Int16, Int32, Int64, Mailbox, Mem, Module, Normal, Not, Of, Proc,
    Pub, Recv, Ref, Region, Return, Self_, Send, Spawn, String, Struct,
    Trait, True, Type, Underscore, Unit, Use, Where,

    // Literals
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(Vec<u8>),  // Byte vector to support \xNN for UTF-8 lead byte matching
    CharLiteral(char),

    // Identifiers
    Identifier(String),

    // Operators (15 total)
    Plus, Minus, Star, Slash, Percent,         // Arithmetic
    EqualEqual, BangEqual,                     // Equality
    Less, LessEqual, Greater, GreaterEqual,    // Comparison
    Ampersand, And, Or, Bang,                  // Reference/Logical
    Equal, LeftArrow, RightArrow,              // Assignment/Arrow
    DoubleColon, Colon,                        // Type operators

    // Punctuation
    LeftParen, RightParen,                     // ( )
    LeftBrace, RightBrace,                     // { }
    LeftBracket, RightBracket,                 // [ ]
    Comma, Semicolon, Dot, Pipe, At,           // , ; . | @
    Question,                                  // ?  (ref? optional-reference types)

    // Special
    EOF,
}

impl TokenKind {
    /// Check if this token kind is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self,
            TokenKind::Actor | TokenKind::ActorRef | TokenKind::As |
            TokenKind::Atomic | TokenKind::Bool | TokenKind::Buf |
            TokenKind::Case | TokenKind::Cast | TokenKind::Char | TokenKind::Concurrency |
            TokenKind::CoreId | TokenKind::CoreSet | TokenKind::DeviceIO | TokenKind::Do | TokenKind::Effect | TokenKind::EfficiencyCores | TokenKind::Includes | TokenKind::PerformanceCores |
            TokenKind::Else | TokenKind::End | TokenKind::Enum |
            TokenKind::Export | TokenKind::False | TokenKind::Fn |
            TokenKind::For | TokenKind::From |             TokenKind::If | TokenKind::Impl |
            TokenKind::Import | TokenKind::Int8 | TokenKind::Int16 | TokenKind::Int32 | TokenKind::Int64 | TokenKind::Float16 | TokenKind::Float32 | TokenKind::Float64 |
            TokenKind::Mailbox | TokenKind::Mem | TokenKind::Module |
            TokenKind::Normal | TokenKind::Not | TokenKind::Of | TokenKind::Proc |
            TokenKind::Pub | TokenKind::Recv | TokenKind::Ref |
            TokenKind::Region | TokenKind::Return | TokenKind::Self_ |
            TokenKind::Send | TokenKind::Spawn | TokenKind::String |
            TokenKind::Struct | TokenKind::Trait | TokenKind::True |
            TokenKind::Type | TokenKind::Underscore | TokenKind::Unit |
            TokenKind::Use | TokenKind::Where
        )
    }

    /// Check if this token kind is a literal
    pub fn is_literal(&self) -> bool {
        matches!(self,
            TokenKind::IntegerLiteral(_) |
            TokenKind::FloatLiteral(_) |
            TokenKind::StringLiteral(_) |
            TokenKind::CharLiteral(_) |
            TokenKind::True | TokenKind::False
        )
    }
}

/// Lexer performs lexical analysis on Silica source code
pub struct Lexer {
    source: String,
    chars: Vec<char>,
    position: usize,
    current_char_index: usize,
    line: usize,
    column: usize,
    file: String,
}

impl Lexer {
    /// Create a new lexer for the given source code
    pub fn new(source: String, file: String) -> Lexer {
        let chars: Vec<char> = source.chars().collect();
        Lexer {
            source,
            chars,
            position: 0,
            current_char_index: 0,
            line: 1,
            column: 1,
            file,
        }
    }

    /// Tokenize the entire source code into a vector of tokens
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            match self.next_token()? {
                Some(token) => {
                    // eprintln!("DEBUG LEXER: Token {:?} '{}' at {}:{}", token.kind, token.lexeme, token.location.line, token.location.column);
                    let is_eof = matches!(token.kind, TokenKind::EOF);
                    tokens.push(token);
                    if is_eof {
                        break;
                    }
                }
                None => break,
            }
        }

        Ok(tokens)
    }

    /// Get the next token from the source
    fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace_and_comments()?;

        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );

        match self.peek_char() {
            Some(c) => match c {
                '(' => self.make_token(TokenKind::LeftParen, "("),
                ')' => self.make_token(TokenKind::RightParen, ")"),
                '{' => self.make_token(TokenKind::LeftBrace, "{"),
                '}' => self.make_token(TokenKind::RightBrace, "}"),
                '[' => self.make_token(TokenKind::LeftBracket, "["),
                ']' => self.make_token(TokenKind::RightBracket, "]"),
                ',' => self.make_token(TokenKind::Comma, ","),
                ';' => self.make_token(TokenKind::Semicolon, ";"),
                '.' => self.make_token(TokenKind::Dot, "."),
                '|' => self.make_token(TokenKind::Pipe, "|"),
                '@' => self.make_token(TokenKind::At, "@"),
                '?' => self.make_token(TokenKind::Question, "?"),
                ':' => self.read_colon_or_double_colon(),
                '=' => self.read_equal_or_double_equal(),
                '!' => self.read_bang_or_bang_equal(),
                '<' => self.read_less_or_less_equal_or_left_arrow(),
                '>' => self.read_greater_or_greater_equal(),
                '+' => self.make_token(TokenKind::Plus, "+"),
                '-' => self.read_minus_or_arrow(),
                '*' => self.make_token(TokenKind::Star, "*"),
                '/' => self.make_token(TokenKind::Slash, "/"),
                '%' => self.make_token(TokenKind::Percent, "%"),
                '&' => self.make_token(TokenKind::Ampersand, "&"),
                'o' => self.read_or(),
                '"' => self.read_string(),
                '\'' => self.read_char(),
                '0'..='9' => self.read_number(),
                'a'..='z' | 'A'..='Z' | '_' => self.read_identifier_or_keyword(),
                _ => {
                    let metadata = self.build_error_metadata("E0001", &start_location, Some("spec:§2.1"))
                        .build();
                    lexer_error_with_metadata(
                        start_location,
                        format!("Unexpected character: {}", c),
                        metadata,
                    )
                },
            },
            None => Ok(Some(Token::new(TokenKind::EOF, "".to_string(), start_location))),
        }
    }

    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) -> Result<()> {
        loop {
            match self.peek_char() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    // Only advance(); it updates line/column. Do not update line/column here
                    // or we double-count newlines and corrupt position tracking.
                    self.advance();
                }
                Some('/') => {
                    if self.peek_next_char() == Some('/') {
                        self.skip_line_comment()?;
                    } else {
                        break;
                    }
                }
                Some('-') => {
                    if self.peek_next_char() == Some('-') {
                        self.skip_line_comment()?;
                    } else {
                        break;
                    }
                }
                Some('{') => {
                    if self.peek_next_char() == Some('-') {
                        self.skip_block_comment()?;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Skip line comments (-- ...)
    fn skip_line_comment(&mut self) -> Result<()> {
        self.advance(); // skip first -
        self.advance(); // skip second -

        while let Some(c) = self.peek_char() {
            self.advance();
            if c == '\n' {
                break;
            }
        }
        Ok(())
    }

    /// Skip block comments ({- ... -})
    fn skip_block_comment(&mut self) -> Result<()> {
        self.advance(); // skip {
        self.advance(); // skip -

        let mut nesting_level = 1;

        while nesting_level > 0 {
            match (self.peek_char(), self.peek_next_char()) {
                (Some('-'), Some('}')) => {
                    self.advance();
                    self.advance();
                    nesting_level -= 1;
                }
                (Some('{'), Some('-')) => {
                    self.advance();
                    self.advance();
                    nesting_level += 1;
                }
                (Some('\n'), _) => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                    debug_assert!(
                        self.line <= MAX_SANE_LINE,
                        "lexer position tracking: line overflow in block comment line={}",
                        self.line
                    );
                }
                (Some(_), _) => {
                    self.advance();
                }
                (None, _) => {
                    let location = SourceLocation::new(self.file.clone(), self.line, self.column, self.position);
                    let metadata = self.build_error_metadata("E0006", &location, Some("spec:§2.3.2"))
                        .build();
                    return lexer_error_with_metadata(
                        location,
                        "Unterminated block comment".to_string(),
                        metadata,
                    );
                }
            }
        }
        Ok(())
    }

    /// Read identifiers and keywords
    fn read_identifier_or_keyword(&mut self) -> Result<Option<Token>> {
        let start = self.position;
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );

        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let lexeme = &self.source[start..self.position];
        let token_kind = self.keyword_or_identifier(lexeme);

        Ok(Some(Token::new(token_kind, lexeme.to_string(), start_location)))
    }

    /// Convert string to keyword or identifier token
    fn keyword_or_identifier(&self, s: &str) -> TokenKind {
        match s {
            "actor" => TokenKind::Actor,
            "actor_ref" => TokenKind::ActorRef,
            "and" => TokenKind::And,
            "atomic" => TokenKind::Atomic,
            "or" => TokenKind::Or,
            "core_id" => TokenKind::CoreId,
            "core_set" => TokenKind::CoreSet,
            "efficiency_cores" => TokenKind::EfficiencyCores,
            "performance_cores" => TokenKind::PerformanceCores,
            "boolean" => TokenKind::Bool,
            "buf" => TokenKind::Buf,
            "case" => TokenKind::Case,
            "cast" => TokenKind::Cast,
            "char" => TokenKind::Char,
            "concurrency" => TokenKind::Concurrency,
            "device_io" => TokenKind::DeviceIO,
            "do" => TokenKind::Do,
            "effect" => TokenKind::Effect,
            "as" => TokenKind::As,
            "else" => TokenKind::Else,
            "end" => TokenKind::End,
            "enum" => TokenKind::Enum,
            "export" => TokenKind::Export,
            "false" => TokenKind::False,
            "fn" => TokenKind::Fn,
            "for" => TokenKind::For,
            "from" => TokenKind::From,
            "if" => TokenKind::If,
            "impl" => TokenKind::Impl,
            "includes" => TokenKind::Includes,
            "import" => TokenKind::Import,
            "int8" => TokenKind::Int8,
            "int16" => TokenKind::Int16,
            "int32" => TokenKind::Int32,
            "int64" => TokenKind::Int64,
            "float16" => TokenKind::Float16,
            "float32" => TokenKind::Float32,
            "float64" => TokenKind::Float64,
            "mailbox" => TokenKind::Mailbox,
            "mem" => TokenKind::Mem,
            "module" => TokenKind::Module,
            "normal" => TokenKind::Normal,
            "not" => TokenKind::Not,
            "of" => TokenKind::Of,
            "proc" => TokenKind::Proc,
            "pub" => TokenKind::Pub,
            "recv" => TokenKind::Recv,
            "ref" => TokenKind::Ref,
            "region" => TokenKind::Region,
            "return" => TokenKind::Return,
            "self" => TokenKind::Self_,
            "send" => TokenKind::Send,
            "spawn" => TokenKind::Spawn,
            "string" => TokenKind::String,
            "struct" => TokenKind::Struct,
            "trait" => TokenKind::Trait,
            "true" => TokenKind::True,
            "type" => TokenKind::Type,
            "underscore" => TokenKind::Underscore,
            "unit" => TokenKind::Unit,
            "use" => TokenKind::Use,
            "where" => TokenKind::Where,
            _ => TokenKind::Identifier(s.to_string()),
        }
    }

    /// Read integer literals
    fn read_number(&mut self) -> Result<Option<Token>> {
        let start = self.position;
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );

        // Handle hex literals
        if self.peek_char() == Some('0') && self.peek_next_char() == Some('x') {
            self.advance(); // skip '0'
            self.advance(); // skip 'x'

            while let Some(c) = self.peek_char() {
                if c.is_ascii_hexdigit() {
                    self.advance();
                } else {
                    break;
                }
            }

            let hex_str = &self.source[start + 2..self.position];
            match i64::from_str_radix(hex_str, 16) {
                Ok(value) => Ok(Some(Token::new(
                    TokenKind::IntegerLiteral(value),
                    self.source[start..self.position].to_string(),
                    start_location,
                ))),
                Err(_) => {
                    let metadata = self.build_error_metadata("E0005", &start_location, Some("spec:§2.2.3"))
                        .suggestion("Check that hex digits are valid (0-9, a-f, A-F)".to_string())
                        .build();
                    lexer_error_with_metadata(
                        start_location,
                        format!("Invalid hex literal: {}", hex_str),
                        metadata,
                    )
                },
            }
        }
        // Handle binary literals
        else if self.peek_char() == Some('0') && self.peek_next_char() == Some('b') {
            self.advance(); // skip '0'
            self.advance(); // skip 'b'

            while let Some(c) = self.peek_char() {
                if c == '0' || c == '1' {
                    self.advance();
                } else {
                    break;
                }
            }

            let bin_str = &self.source[start + 2..self.position];
            match i64::from_str_radix(bin_str, 2) {
                Ok(value) => Ok(Some(Token::new(
                    TokenKind::IntegerLiteral(value),
                    self.source[start..self.position].to_string(),
                    start_location,
                ))),
                Err(_) => {
                    let metadata = self.build_error_metadata("E0005", &start_location, Some("spec:§2.2.3"))
                        .suggestion("Check that binary digits are valid (0 or 1)".to_string())
                        .build();
                    lexer_error_with_metadata(
                        start_location,
                        format!("Invalid binary literal: {}", bin_str),
                        metadata,
                    )
                },
            }
        }
        // Handle decimal literals (integer or float)
        else {
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check if this is a float literal (has decimal point followed by digits)
            if self.peek_char() == Some('.') {
                let next_char = self.peek_next_char();
                if next_char.map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    // This is a float literal
                    self.advance(); // skip '.'
                    while let Some(c) = self.peek_char() {
                        if c.is_ascii_digit() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    // Check for scientific notation (e/E followed by optional +/- and digits)
                    if let Some(c) = self.peek_char() {
                        if c == 'e' || c == 'E' {
                            self.advance(); // skip 'e' or 'E'
                            // Optional sign
                            if let Some(sign) = self.peek_char() {
                                if sign == '+' || sign == '-' {
                                    self.advance(); // skip sign
                                }
                            }
                            // Exponent digits
                            let mut has_exponent_digits = false;
                            while let Some(c) = self.peek_char() {
                                if c.is_ascii_digit() {
                                    self.advance();
                                    has_exponent_digits = true;
                                } else {
                                    break;
                                }
                            }
                            // If we saw 'e'/'E' but no digits, that's an error
                            if !has_exponent_digits {
                                let metadata = self.build_error_metadata("E0005", &start_location, Some("spec:§2.2.3"))
                                    .suggestion("Scientific notation requires digits after 'e' or 'E'".to_string())
                                    .build();
                                return lexer_error_with_metadata(
                                    start_location,
                                    "Invalid float literal: missing exponent digits".to_string(),
                                    metadata,
                                );
                            }
                        }
                    }
                    
                    let float_str = &self.source[start..self.position];
                    match float_str.parse::<f64>() {
                        Ok(value) => Ok(Some(Token::new(
                            TokenKind::FloatLiteral(value),
                            float_str.to_string(),
                            start_location,
                        ))),
                        Err(_) => {
                            let metadata = self.build_error_metadata("E0005", &start_location, Some("spec:§2.2.3"))
                                .suggestion("Ensure the float literal is valid".to_string())
                                .build();
                            lexer_error_with_metadata(
                                start_location,
                                format!("Invalid float literal: {}", float_str),
                                metadata,
                            )
                        },
                    }
                } else {
                    // Just a dot, not part of float - parse as integer
                    let num_str = &self.source[start..self.position];
                    match num_str.parse::<i64>() {
                        Ok(value) => Ok(Some(Token::new(
                            TokenKind::IntegerLiteral(value),
                            num_str.to_string(),
                            start_location,
                        ))),
                        Err(_) => {
                            let metadata = self.build_error_metadata("E0005", &start_location, Some("spec:§2.2.3"))
                                .suggestion("Ensure the number is within valid range for i64".to_string())
                                .build();
                            lexer_error_with_metadata(
                                start_location,
                                format!("Invalid integer literal: {}", num_str),
                                metadata,
                            )
                        },
                    }
                }
            } else {
                // Integer literal
                let num_str = &self.source[start..self.position];
                match num_str.parse::<i64>() {
                    Ok(value) => Ok(Some(Token::new(
                        TokenKind::IntegerLiteral(value),
                        num_str.to_string(),
                        start_location,
                    ))),
                    Err(_) => {
                        let metadata = self.build_error_metadata("E0005", &start_location, Some("spec:§2.2.3"))
                            .suggestion("Ensure the number is within valid range for i64".to_string())
                            .build();
                        lexer_error_with_metadata(
                            start_location,
                            format!("Invalid integer literal: {}", num_str),
                            metadata,
                        )
                    },
                }
            }
        }
    }

    /// Read string literals
    fn read_string(&mut self) -> Result<Option<Token>> {
        let start = self.position;
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );

        self.advance(); // skip opening quote

        let mut result = Vec::new();
        let mut escaped = false;

        while let Some(c) = self.peek_char() {
            self.advance();

            if escaped {
                match c {
                    'n' => result.extend_from_slice("\n".as_bytes()),
                    't' => result.extend_from_slice("\t".as_bytes()),
                    'r' => result.extend_from_slice("\r".as_bytes()),
                    '\\' => result.extend_from_slice("\\".as_bytes()),
                    '"' => result.extend_from_slice("\"".as_bytes()),
                    '\'' => result.extend_from_slice("'".as_bytes()),
                    'x' => {
                        let mut hex_str = String::new();
                        for _ in 0..2 {
                            if let Some(d) = self.peek_char() {
                                if d.is_ascii_hexdigit() {
                                    hex_str.push(d);
                                    self.advance();
                                } else { break; }
                            }
                        }
                        if hex_str.len() == 2 {
                            result.push(u8::from_str_radix(&hex_str, 16).unwrap_or(0));
                        } else {
                            let metadata = self.build_error_metadata("E0002", &start_location, Some("spec:§2.2.3"))
                                .suggestion("\\x must be followed by exactly two hex digits (0-9, a-f, A-F)".to_string())
                                .build();
                            return lexer_error_with_metadata(
                                start_location,
                                format!("Invalid hex escape: \\x{} (expected 2 hex digits)", hex_str),
                                metadata,
                            );
                        }
                    }
                    _ => {
                        let metadata = self.build_error_metadata("E0002", &start_location, Some("spec:§2.2.3"))
                            .suggestion("Valid escape sequences are: \\n, \\t, \\r, \\\\, \\\", \\', \\xNN".to_string())
                            .build();
                        return lexer_error_with_metadata(
                            start_location,
                            format!("Invalid escape sequence: \\{}", c),
                            metadata,
                        );
                    }
                }
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                return Ok(Some(Token::new(
                    TokenKind::StringLiteral(result),
                    self.source[start..self.position].to_string(),
                    start_location,
                )));
            } else {
                result.extend_from_slice(c.to_string().as_bytes());
            }
        }

        {
            let metadata = self.build_error_metadata("E0003", &start_location, Some("spec:§2.2.3"))
                .suggestion("Add closing double quote (\") to terminate the string literal".to_string())
                .build();
            lexer_error_with_metadata(
                start_location,
                "Unterminated string literal".to_string(),
                metadata,
            )
        }
    }

    /// Read character literals
    fn read_char(&mut self) -> Result<Option<Token>> {
        let start = self.position;
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );

        self.advance(); // skip opening quote

        let c = match self.peek_char() {
            Some('\\') => {
                self.advance(); // skip backslash
                match self.peek_char() {
                    Some('n') => {
                        self.advance();
                        '\n'
                    }
                    Some('t') => {
                        self.advance();
                        '\t'
                    }
                    Some('r') => {
                        self.advance();
                        '\r'
                    }
                    Some('\\') => {
                        self.advance();
                        '\\'
                    }
                    Some('"') => {
                        self.advance();
                        '"'
                    }
                    Some('\'') => {
                        self.advance();
                        '\''
                    }
                    Some(c) => {
                        let metadata = self.build_error_metadata("E0002", &start_location, Some("spec:§2.2.3"))
                            .suggestion("Valid escape sequences are: \\n, \\t, \\r, \\\\, \\\", \\'".to_string())
                            .build();
                        return lexer_error_with_metadata(
                            start_location,
                            format!("Invalid escape sequence: \\{}", c),
                            metadata,
                        );
                    }
                    None => {
                        let metadata = self.build_error_metadata("E0004", &start_location, Some("spec:§2.2.3"))
                            .suggestion("Add closing single quote (') to terminate the character literal".to_string())
                            .build();
                        return lexer_error_with_metadata(
                            start_location,
                            "Unexpected end of file in character literal".to_string(),
                            metadata,
                        );
                    }
                }
            }
            Some(c) => {
                self.advance();
                c
            }
            None => {
                let metadata = self.build_error_metadata("E0004", &start_location, Some("spec:§2.2.3"))
                    .suggestion("Add closing single quote (') to terminate the character literal".to_string())
                    .build();
                return lexer_error_with_metadata(
                    start_location,
                    "Unexpected end of file in character literal".to_string(),
                    metadata,
                );
            }
        };

        match self.peek_char() {
            Some('\'') => {
                self.advance(); // skip closing quote
                Ok(Some(Token::new(
                    TokenKind::CharLiteral(c),
                    self.source[start..self.position].to_string(),
                    start_location,
                )))
            }
            _ => {
                let metadata = self.build_error_metadata("E0004", &start_location, Some("spec:§2.2.3"))
                    .suggestion("Character literals must contain exactly one character. Use single quotes: 'a'".to_string())
                    .build();
                lexer_error_with_metadata(
                    start_location,
                    "Expected closing quote in character literal".to_string(),
                    metadata,
                )
            },
        }
    }

    // Operator reading methods
    fn read_colon_or_double_colon(&mut self) -> Result<Option<Token>> {
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance(); // skip first :
        if self.peek_char() == Some(':') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::DoubleColon, "::", start_location)
        } else {
            self.make_token_at_current_pos(TokenKind::Colon, ":", start_location)
        }
    }

    fn read_equal_or_double_equal(&mut self) -> Result<Option<Token>> {
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance(); // skip first =
        if self.peek_char() == Some('=') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::EqualEqual, "==", start_location)
        } else {
            self.make_token_at_current_pos(TokenKind::Equal, "=", start_location)
        }
    }

    fn read_bang_or_bang_equal(&mut self) -> Result<Option<Token>> {
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance(); // skip !
        if self.peek_char() == Some('=') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::BangEqual, "!=", start_location)
        } else {
            self.make_token_at_current_pos(TokenKind::Bang, "!", start_location)
        }
    }

    fn read_less_or_less_equal_or_left_arrow(&mut self) -> Result<Option<Token>> {
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance(); // skip <
        match self.peek_char() {
            Some('=') => {
                self.advance();
                self.make_token_at_current_pos(TokenKind::LessEqual, "<=", start_location)
            }
            Some('-') => {
                self.advance();
                self.make_token_at_current_pos(TokenKind::LeftArrow, "<-", start_location)
            }
            _ => self.make_token_at_current_pos(TokenKind::Less, "<", start_location),
        }
    }

    fn read_greater_or_greater_equal(&mut self) -> Result<Option<Token>> {
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance(); // skip >
        if self.peek_char() == Some('=') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::GreaterEqual, ">=", start_location)
        } else {
            self.make_token_at_current_pos(TokenKind::Greater, ">", start_location)
        }
    }

    fn read_minus_or_arrow(&mut self) -> Result<Option<Token>> {
        let start_location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance(); // skip -
        if self.peek_char() == Some('>') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::RightArrow, "->", start_location)
        } else {
            self.make_token_at_current_pos(TokenKind::Minus, "-", start_location)
        }
    }

    fn read_and(&mut self) -> Result<Option<Token>> {
        // 'and' is a keyword, not an operator
        self.read_identifier_or_keyword()
    }

    fn read_or(&mut self) -> Result<Option<Token>> {
        // 'or' is a keyword, not an operator
        self.read_identifier_or_keyword()
    }

    /// Helper methods
    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.current_char_index).copied()
    }

    fn peek_next_char(&self) -> Option<char> {
        self.chars.get(self.current_char_index + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(c) = self.chars.get(self.current_char_index) {
            self.position += c.len_utf8();
            self.column += 1;
            if *c == '\n' {
                self.line += 1;
                self.column = 1;
                debug_assert!(
                    self.line <= MAX_SANE_LINE,
                    "lexer position tracking: line overflow line={}",
                    self.line
                );
            }
            self.current_char_index += 1;
            Some(*c)
        } else {
            None
        }
    }

    /// Diagnostic: log when position is out of expected range (indicates position-tracking bug).
    fn check_position_sanity(&self, location: &SourceLocation) {
        if location.line < 1 || location.line > 10000 {
            eprintln!(
                "[lexer] position tracking: line out of range file={} line={} column={} (expected 1..=10000)",
                location.file, location.line, location.column
            );
        }
        debug_assert!(
            location.line >= 1 && location.line <= MAX_SANE_LINE,
            "lexer token position out of range: file={} line={} column={}",
            location.file, location.line, location.column
        );
    }

    fn make_token(&mut self, kind: TokenKind, lexeme: &str) -> Result<Option<Token>> {
        let location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.check_position_sanity(&location);
        self.advance();
        Ok(Some(Token::new(kind, lexeme.to_string(), location)))
    }

    /// Create a token using the start location (captured before consuming the lexeme).
    /// Callers must pass the SourceLocation from before any advance() that consumed this token.
    fn make_token_at_current_pos(&mut self, kind: TokenKind, lexeme: &str, start_location: SourceLocation) -> Result<Option<Token>> {
        self.check_position_sanity(&start_location);
        // Don't advance - characters already consumed
        Ok(Some(Token::new(kind, lexeme.to_string(), start_location)))
    }

    /// Create error metadata builder with surrounding code context
    fn build_error_metadata(&self, error_code: &str, location: &SourceLocation, spec_section: Option<&str>) -> crate::errors::ErrorMetadataBuilder {
        use crate::errors::{ErrorMetadataBuilder, ErrorSeverity, extract_surrounding_code};
        
        let mut builder = ErrorMetadataBuilder::new(error_code.to_string())
            .severity(ErrorSeverity::Error);
        
        // Add surrounding code context
        if let Some(context) = extract_surrounding_code(&self.source, location, 3) {
            builder = builder.surrounding_code(context);
        }
        
        // Add specification reference
        if let Some(section) = spec_section {
            // Remove "spec:" prefix if present, store just "§X.Y"
            let clean_section = if section.starts_with("spec:") {
                section.strip_prefix("spec:").unwrap_or(section)
            } else {
                section
            };
            builder = builder.specification(clean_section.to_string(), None);
        }
        
        builder
    }
}
