use crate::errors::{Result, SourceLocation, lexer_error};

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
    // Keywords (26 total)
    Actor, ActorRef, As, Atomic, Bool, Buf, Case, Char, Concurrency, CoreId, CoreSet, DeviceIO, EfficiencyCores, PerformanceCores,
    Do, Effect, Else, End, Enum, Export, False, Fn, For, From, If,
    Impl, Import, Int, Let, Mailbox, Mem, Module, Normal, Of, Proc,
    Pub, Recv, Ref, Region, Return, Self_, Send, Spawn, String, Struct,
    Trait, True, Type, Underscore, Unit, Use, Where,

    // Literals
    IntegerLiteral(i64),
    StringLiteral(String),
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
    Comma, Semicolon, Dot, Pipe,               // , ; . |

    // Special
    EOF,
}

impl TokenKind {
    /// Check if this token kind is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self,
            TokenKind::Actor | TokenKind::ActorRef | TokenKind::As |
            TokenKind::Atomic | TokenKind::Bool | TokenKind::Buf |
            TokenKind::Case | TokenKind::Char | TokenKind::Concurrency |
            TokenKind::CoreId | TokenKind::CoreSet | TokenKind::DeviceIO | TokenKind::Do | TokenKind::Effect | TokenKind::EfficiencyCores | TokenKind::PerformanceCores |
            TokenKind::Else | TokenKind::End | TokenKind::Enum |
            TokenKind::Export | TokenKind::False | TokenKind::Fn |
            TokenKind::For | TokenKind::From |             TokenKind::If | TokenKind::Impl |
            TokenKind::Import | TokenKind::Int | TokenKind::Let |
            TokenKind::Mailbox | TokenKind::Mem | TokenKind::Module |
            TokenKind::Normal | TokenKind::Of | TokenKind::Proc |
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
                _ => lexer_error(
                    start_location,
                    format!("Unexpected character: {}", c),
                ),
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
                    self.line += 1;
                    self.column = 1;
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
                }
                (Some(_), _) => {
                    self.advance();
                }
                (None, _) => {
                    return lexer_error(
                        SourceLocation::new(self.file.clone(), self.line, self.column, self.position),
                        "Unterminated block comment".to_string(),
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
            "import" => TokenKind::Import,
            "int" => TokenKind::Int,
            "let" => TokenKind::Let,
            "mailbox" => TokenKind::Mailbox,
            "mem" => TokenKind::Mem,
            "module" => TokenKind::Module,
            "normal" => TokenKind::Normal,
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
                Err(_) => lexer_error(
                    start_location,
                    format!("Invalid hex literal: {}", hex_str),
                ),
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
                Err(_) => lexer_error(
                    start_location,
                    format!("Invalid binary literal: {}", bin_str),
                ),
            }
        }
        // Handle decimal literals
        else {
            while let Some(c) = self.peek_char() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }

            let num_str = &self.source[start..self.position];
            match num_str.parse::<i64>() {
                Ok(value) => Ok(Some(Token::new(
                    TokenKind::IntegerLiteral(value),
                    num_str.to_string(),
                    start_location,
                ))),
                Err(_) => lexer_error(
                    start_location,
                    format!("Invalid integer literal: {}", num_str),
                ),
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

        let mut result = String::new();
        let mut escaped = false;

        while let Some(c) = self.peek_char() {
            self.advance();

            if escaped {
                match c {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '\'' => result.push('\''),
                    _ => {
                        return lexer_error(
                            start_location,
                            format!("Invalid escape sequence: \\{}", c),
                        );
                    }
                }
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                // End of string
                return Ok(Some(Token::new(
                    TokenKind::StringLiteral(result),
                    self.source[start..self.position].to_string(),
                    start_location,
                )));
            } else {
                result.push(c);
            }
        }

        lexer_error(
            start_location,
            "Unterminated string literal".to_string(),
        )
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
                        return lexer_error(
                            start_location,
                            format!("Invalid escape sequence: \\{}", c),
                        );
                    }
                    None => {
                        return lexer_error(
                            start_location,
                            "Unexpected end of file in character literal".to_string(),
                        );
                    }
                }
            }
            Some(c) => {
                self.advance();
                c
            }
            None => {
                return lexer_error(
                    start_location,
                    "Unexpected end of file in character literal".to_string(),
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
            _ => lexer_error(
                start_location,
                "Expected closing quote in character literal".to_string(),
            ),
        }
    }

    // Operator reading methods
    fn read_colon_or_double_colon(&mut self) -> Result<Option<Token>> {
        self.advance(); // skip first :
        if self.peek_char() == Some(':') {
            self.advance();
            self.make_token(TokenKind::DoubleColon, "::")
        } else {
            self.make_token(TokenKind::Colon, ":")
        }
    }

    fn read_equal_or_double_equal(&mut self) -> Result<Option<Token>> {
        let start_pos = self.position;
        self.advance(); // skip first =
        if self.peek_char() == Some('=') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::EqualEqual, "==")
        } else {
            self.make_token_at_current_pos(TokenKind::Equal, "=")
        }
    }

    fn read_bang_or_bang_equal(&mut self) -> Result<Option<Token>> {
        let start_pos = self.position;
        self.advance(); // skip !
        if self.peek_char() == Some('=') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::BangEqual, "!=")
        } else {
            self.make_token_at_current_pos(TokenKind::Bang, "!")
        }
    }

    fn read_less_or_less_equal_or_left_arrow(&mut self) -> Result<Option<Token>> {
        let start_pos = self.position;
        self.advance(); // skip <
        match self.peek_char() {
            Some('=') => {
                self.advance();
                self.make_token_at_current_pos(TokenKind::LessEqual, "<=")
            }
            Some('-') => {
                self.advance();
                self.make_token_at_current_pos(TokenKind::LeftArrow, "<-")
            }
            _ => self.make_token_at_current_pos(TokenKind::Less, "<"),
        }
    }

    fn read_greater_or_greater_equal(&mut self) -> Result<Option<Token>> {
        let start_pos = self.position;
        self.advance(); // skip >
        if self.peek_char() == Some('=') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::GreaterEqual, ">=")
        } else {
            self.make_token_at_current_pos(TokenKind::Greater, ">")
        }
    }

    fn read_minus_or_arrow(&mut self) -> Result<Option<Token>> {
        let start_pos = self.position;
        self.advance(); // skip -
        if self.peek_char() == Some('>') {
            self.advance();
            self.make_token_at_current_pos(TokenKind::RightArrow, "->")
        } else {
            self.make_token_at_current_pos(TokenKind::Minus, "-")
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
            }
            self.current_char_index += 1;
            Some(*c)
        } else {
            None
        }
    }

    fn make_token(&mut self, kind: TokenKind, lexeme: &str) -> Result<Option<Token>> {
        let location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position,
        );
        self.advance();
        Ok(Some(Token::new(kind, lexeme.to_string(), location)))
    }

    fn make_token_at_current_pos(&mut self, kind: TokenKind, lexeme: &str) -> Result<Option<Token>> {
        let location = SourceLocation::new(
            self.file.clone(),
            self.line,
            self.column,
            self.position - lexeme.len(),
        );
        // Don't advance - characters already consumed
        Ok(Some(Token::new(kind, lexeme.to_string(), location)))
    }
}
