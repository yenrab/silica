use crate::ast::*;
use crate::errors::{Result, SourceLocation, parse_error};
use crate::lexer::{Lexer, Token, TokenKind};

/// Parser performs recursive descent parsing of Silica source code
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
        }
    }

    /// Parse the entire token stream into a Program AST
    pub fn parse(&mut self) -> Result<Program> {
        let mut declarations = Vec::new();
        let location = self.peek().location.clone();

        while !self.is_at_end() {
            if self.match_token(TokenKind::EOF) {
                break;
            }
            declarations.push(self.declaration()?);
        }

        Ok(Program {
            declarations,
            location,
        })
    }

    /// Parse a top-level declaration
    fn declaration(&mut self) -> Result<Declaration> {
        if self.match_token(TokenKind::Fn) {
            self.function_declaration().map(Declaration::Function)
        } else if self.match_token(TokenKind::Type) {
            // For now, only support type aliases
            self.type_alias_declaration().map(Declaration::TypeAlias)
        } else if self.match_token(TokenKind::Effect) {
            self.effect_declaration().map(Declaration::Effect)
        } else if self.match_token(TokenKind::Module) {
            self.module_declaration().map(Declaration::Module)
        } else if self.match_token(TokenKind::Import) {
            self.import_declaration().map(Declaration::Import)
        } else if self.match_token(TokenKind::Export) {
            self.export_declaration().map(Declaration::Export)
        } else if self.match_token(TokenKind::Struct) {
            self.struct_declaration().map(Declaration::Struct)
        } else if self.match_token(TokenKind::Enum) {
            self.enum_declaration().map(Declaration::Enum)
        } else if self.match_token(TokenKind::Trait) {
            self.trait_declaration().map(Declaration::Trait)
        } else if self.match_token(TokenKind::Impl) {
            self.impl_declaration().map(Declaration::Impl)
        } else if self.match_token(TokenKind::Use) {
            // Legacy support for use as module declaration
            self.module_declaration().map(Declaration::Module)
        } else {
            parse_error(
                self.peek().location.clone(),
                "Expected declaration (fn, type, effect, module, import, export, struct, enum, trait, or impl)".to_string(),
            )
        }
    }

    /// Parse a function declaration: fn name(params) [: return_type] { body }
    fn function_declaration(&mut self) -> Result<FunctionDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected function name")?;

        self.consume(TokenKind::LeftParen, "Expected '(' after function name")?;
        let parameters = self.parameter_list()?;
        self.consume(TokenKind::RightParen, "Expected ')' after parameters")?;

        let return_type = if self.match_token(TokenKind::RightArrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let effects = if self.match_token(TokenKind::Proc) {
            self.consume(TokenKind::LeftBracket, "Expected '[' after 'proc'")?;
            let effects = self.effect_list()?;
            self.consume(TokenKind::RightBracket, "Expected ']' after effects")?;
            effects
        } else {
            Vec::new()
        };

        self.consume(TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.expression()?;
        self.consume(TokenKind::RightBrace, "Expected '}' after function body")?;

        Ok(FunctionDecl {
            name,
            parameters,
            return_type,
            body,
            effects,
            location,
        })
    }

    /// Parse parameter list: (param1: Type1, param2: Type2, ...)
    fn parameter_list(&mut self) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        if !self.check(TokenKind::RightParen) {
            loop {
                let param = self.parameter()?;
                parameters.push(param);

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }

        Ok(parameters)
    }

    /// Parse a single parameter: name: Type or self
    fn parameter(&mut self) -> Result<Parameter> {
        let location = self.peek().location.clone();
        let name = self.consume_identifier("Expected parameter name")?;

        // Check if this is a self parameter (no type annotation)
        let type_ = if self.match_token(TokenKind::Colon) {
            self.parse_type()?
        } else {
            // For self parameters, use a placeholder type
            // In a real implementation, this would be resolved to the implementing type
            Type::Named("Self".to_string())
        };

        Ok(Parameter {
            name,
            type_,
            location,
        })
    }

    /// Parse a type declaration: type Name = Type
    fn type_declaration(&mut self) -> Result<TypeDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected type name")?;
        self.consume(TokenKind::Equal, "Expected '=' after type name")?;
        let type_ = self.parse_type()?;
        self.consume(TokenKind::Semicolon, "Expected ';' after type definition")?;

        Ok(TypeDecl {
            name,
            type_,
            location,
        })
    }

    /// Parse an effect declaration: effect Name = [effects]
    fn effect_declaration(&mut self) -> Result<EffectDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected effect name")?;
        self.consume(TokenKind::Equal, "Expected '=' after effect name")?;
        self.consume(TokenKind::LeftBracket, "Expected '[' after '='")?;
        let effects = self.effect_list()?;
        self.consume(TokenKind::RightBracket, "Expected ']' after effects")?;
        self.consume(TokenKind::Semicolon, "Expected ';' after effect definition")?;

        Ok(EffectDecl {
            name,
            effects,
            location,
        })
    }

    /// Parse a module declaration: use module path
    fn module_declaration(&mut self) -> Result<ModuleDecl> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::Module, "Expected 'module' after 'use'")?;
        let path = self.module_path()?;
        self.consume(TokenKind::Semicolon, "Expected ';' after module path")?;

        Ok(ModuleDecl {
            path,
            location,
        })
    }

    /// Parse module path: ident::ident::...
    fn module_path(&mut self) -> Result<Vec<String>> {
        let mut path = Vec::new();
        path.push(self.consume_identifier("Expected module name")?);

        while self.match_token(TokenKind::DoubleColon) {
            path.push(self.consume_identifier("Expected module name after '::'")?);
        }

        Ok(path)
    }

    /// Parse import declaration: import module::path [as alias];
    fn import_declaration(&mut self) -> Result<ImportDecl> {
        let location = self.previous().location.clone();
        let path = self.module_path()?;

        let alias = if self.match_token(TokenKind::As) {
            Some(self.consume_identifier("Expected identifier after 'as'")?)
        } else {
            None
        };

        self.consume(TokenKind::Semicolon, "Expected ';' after import")?;

        Ok(ImportDecl {
            path,
            alias,
            location,
        })
    }

    /// Parse export declaration: export identifier;
    fn export_declaration(&mut self) -> Result<ExportDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected identifier after 'export'")?;
        self.consume(TokenKind::Semicolon, "Expected ';' after export")?;

        Ok(ExportDecl {
            name,
            location,
        })
    }

    /// Parse a type expression
    fn parse_type(&mut self) -> Result<Type> {
        if self.match_token(TokenKind::LeftParen) {
            // Tuple type or function type
            if self.match_token(TokenKind::RightParen) {
                // Unit type
                Ok(Type::Unit)
            } else {
                let first_type = self.parse_type()?;
                if self.match_token(TokenKind::Comma) {
                    // Tuple type
                    let mut types = vec![first_type];
                    loop {
                        types.push(self.parse_type()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.consume(TokenKind::RightParen, "Expected ')' after tuple types")?;
                    Ok(Type::Tuple(types))
                } else {
                    // Function type
                    self.consume(TokenKind::RightArrow, "Expected '->' in function type")?;
                    let return_type = self.parse_type()?;
                    self.consume(TokenKind::RightParen, "Expected ')' after function type")?;
                    Ok(Type::Function {
                        parameters: vec![first_type],
                        return_type: Box::new(return_type),
                    })
                }
            }
        } else if self.match_token(TokenKind::Proc) {
            // Process type
            self.consume(TokenKind::LeftBracket, "Expected '[' after 'proc'")?;
            let effects = self.effect_list()?;
            self.consume(TokenKind::RightBracket, "Expected ']' after effects")?;
            let result_type = self.parse_type()?;
            Ok(Type::Process {
                effects,
                result_type: Box::new(result_type),
            })
        } else if self.match_token(TokenKind::LeftBrace) {
            // Record type
            let mut fields = Vec::new();
            while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
                let field_name = self.consume_identifier("Expected field name")?;
                self.consume(TokenKind::Colon, "Expected ':' after field name")?;
                let field_type = self.parse_type()?;
                fields.push((field_name, field_type));

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            self.consume(TokenKind::RightBrace, "Expected '}' after record fields")?;
            Ok(Type::Record(fields))
        } else if self.match_token(TokenKind::Region) {
            // Region type
            self.consume(TokenKind::LeftParen, "Expected '(' after 'region'")?;
            let space = self.memory_space()?;
            self.consume(TokenKind::RightParen, "Expected ')' after memory space")?;
            Ok(Type::Region { space })
        } else if self.match_token(TokenKind::Ref) {
            // Reference type
            self.consume(TokenKind::LeftParen, "Expected '(' after 'ref'")?;
            let region = self.parse_type()?;
            self.consume(TokenKind::Comma, "Expected ',' after region")?;
            let space = self.memory_space()?;
            self.consume(TokenKind::Comma, "Expected ',' after memory space")?;
            let element_type = self.parse_type()?;
            self.consume(TokenKind::RightParen, "Expected ')' after reference type")?;
            Ok(Type::Reference {
                region: Box::new(region),
                space,
                element_type: Box::new(element_type),
            })
        } else if self.match_token(TokenKind::Buf) {
            // Buffer type
            self.consume(TokenKind::LeftParen, "Expected '(' after 'buf'")?;
            let region = self.parse_type()?;
            self.consume(TokenKind::Comma, "Expected ',' after region")?;
            let space = self.memory_space()?;
            self.consume(TokenKind::Comma, "Expected ',' after memory space")?;
            let element_type = self.parse_type()?;
            self.consume(TokenKind::Comma, "Expected ',' after element type")?;
            let capacity = self.consume_integer("Expected buffer capacity")?;
            self.consume(TokenKind::RightParen, "Expected ')' after buffer type")?;
            Ok(Type::Buffer {
                region: Box::new(region),
                space,
                element_type: Box::new(element_type),
                capacity: capacity as usize,
            })
        } else if self.match_token(TokenKind::ActorRef) {
            // Actor reference type
            self.consume(TokenKind::LeftParen, "Expected '(' after 'actor_ref'")?;
            let message_type = self.parse_type()?;
            self.consume(TokenKind::RightParen, "Expected ')' after message type")?;
            Ok(Type::ActorRef {
                message_type: Box::new(message_type),
            })
        } else if self.match_token(TokenKind::Int) {
            Ok(Type::Int)
        } else if self.match_token(TokenKind::Bool) {
            Ok(Type::Bool)
        } else if self.match_token(TokenKind::Char) {
            Ok(Type::Char)
        } else if self.match_token(TokenKind::Unit) {
            Ok(Type::Unit)
        } else if self.match_token(TokenKind::String) {
            Ok(Type::String)
        } else {
            // User-defined type name
            let name = self.consume_identifier("Expected type name")?;
            Ok(Type::Named(name))
        }
    }

    /// Parse memory space
    fn memory_space(&mut self) -> Result<MemorySpace> {
        if self.match_token(TokenKind::Normal) {
            Ok(MemorySpace::Normal)
        } else if self.match_token(TokenKind::Atomic) {
            Ok(MemorySpace::Atomic)
        } else {
            parse_error(
                self.peek().location.clone(),
                "Expected 'normal' or 'atomic' memory space".to_string(),
            )
        }
    }

    /// Parse effect list
    fn effect_list(&mut self) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        if !self.check(TokenKind::RightBracket) {
            loop {
                effects.push(self.parse_effect()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }

        Ok(effects)
    }

    /// Parse a single effect
    fn parse_effect(&mut self) -> Result<Effect> {
        if self.match_token(TokenKind::Mem) {
            self.consume(TokenKind::LeftParen, "Expected '(' after 'mem'")?;
            let space = self.memory_space()?;
            self.consume(TokenKind::RightParen, "Expected ')' after memory space")?;
            Ok(Effect::Memory(space))
        } else if self.match_token(TokenKind::Mailbox) {
            self.consume(TokenKind::LeftParen, "Expected '(' after 'mailbox'")?;
            let msg_type = self.parse_type()?;
            self.consume(TokenKind::RightParen, "Expected ')' after message type")?;
            Ok(Effect::Mailbox(Box::new(msg_type)))
        } else if self.match_token(TokenKind::Concurrency) {
            Ok(Effect::Concurrency)
        } else if self.match_token(TokenKind::Atomic) {
            Ok(Effect::Atomic)
        } else if self.match_token(TokenKind::DeviceIO) {
            Ok(Effect::DeviceIO)
        } else {
            let name = self.consume_identifier("Expected effect name")?;
            Ok(Effect::Named(name))
        }
    }

    /// Parse expression with precedence
    fn expression(&mut self) -> Result<Expression> {
        self.assignment()
    }

    /// Parse assignment expression (lowest precedence)
    fn assignment(&mut self) -> Result<Expression> {
        let expr = self.or()?;

        if self.match_token(TokenKind::LeftArrow) {
            let value = self.assignment()?;
            match expr {
                Expression::Identifier(name) => Ok(Expression::Do(DoExpr {
                    statements: vec![
                        Statement::Bind {
                            pattern: Pattern::Identifier(name),
                            expr: Box::new(value),
                        }
                    ],
                    location: self.previous().location.clone(),
                })),
                _ => parse_error(
                    self.previous().location.clone(),
                    "Invalid assignment target".to_string(),
                ),
            }
        } else {
            Ok(expr)
        }
    }

    /// Parse logical OR
    fn or(&mut self) -> Result<Expression> {
        let mut expr = self.and()?;

        while self.match_token(TokenKind::Or) {
            let operator = BinaryOp::Or;
            let right = self.and()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                location: self.previous().location.clone(),
            });
        }

        Ok(expr)
    }

    /// Parse logical AND
    fn and(&mut self) -> Result<Expression> {
        let mut expr = self.equality()?;

        while self.match_token(TokenKind::And) {
            let operator = BinaryOp::And;
            let right = self.equality()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                location: self.previous().location.clone(),
            });
        }

        Ok(expr)
    }

    /// Parse equality expressions
    fn equality(&mut self) -> Result<Expression> {
        let mut expr = self.comparison()?;

        while self.match_token(TokenKind::EqualEqual) || self.match_token(TokenKind::BangEqual) {
            let operator = if self.previous().kind == TokenKind::EqualEqual {
                BinaryOp::Equal
            } else {
                BinaryOp::NotEqual
            };
            let right = self.comparison()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                location: self.previous().location.clone(),
            });
        }

        Ok(expr)
    }

    /// Parse comparison expressions
    fn comparison(&mut self) -> Result<Expression> {
        let mut expr = self.term()?;

        while self.match_token(TokenKind::Less) || self.match_token(TokenKind::LessEqual) ||
              self.match_token(TokenKind::Greater) || self.match_token(TokenKind::GreaterEqual) {
            let operator = match self.previous().kind {
                TokenKind::Less => BinaryOp::Less,
                TokenKind::LessEqual => BinaryOp::LessEqual,
                TokenKind::Greater => BinaryOp::Greater,
                TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                _ => unreachable!(),
            };
            let right = self.term()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                location: self.previous().location.clone(),
            });
        }

        Ok(expr)
    }

    /// Parse additive expressions
    fn term(&mut self) -> Result<Expression> {
        let mut expr = self.factor()?;

        while self.match_token(TokenKind::Plus) || self.match_token(TokenKind::Minus) {
            let operator = if self.previous().kind == TokenKind::Plus {
                BinaryOp::Add
            } else {
                BinaryOp::Subtract
            };
            let right = self.factor()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                location: self.previous().location.clone(),
            });
        }

        Ok(expr)
    }

    /// Parse multiplicative expressions
    fn factor(&mut self) -> Result<Expression> {
        let mut expr = self.unary()?;

        while self.match_token(TokenKind::Star) || self.match_token(TokenKind::Slash) ||
              self.match_token(TokenKind::Percent) {
            let operator = match self.previous().kind {
                TokenKind::Star => BinaryOp::Multiply,
                TokenKind::Slash => BinaryOp::Divide,
                TokenKind::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
                location: self.previous().location.clone(),
            });
        }

        Ok(expr)
    }

    /// Parse unary expressions
    fn unary(&mut self) -> Result<Expression> {
        if self.match_token(TokenKind::Bang) || self.match_token(TokenKind::Minus) {
            let operator = if self.previous().kind == TokenKind::Bang {
                UnaryOp::Not
            } else {
                UnaryOp::Negate
            };
            let operand = self.unary()?;
            Ok(Expression::Unary(UnaryExpr {
                operator,
                operand: Box::new(operand),
                location: self.previous().location.clone(),
            }))
        } else {
            self.call()
        }
    }

    /// Parse function calls and primary expressions
    fn call(&mut self) -> Result<Expression> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Finish parsing a function call
    fn finish_call(&mut self, callee: Expression) -> Result<Expression> {
        let mut arguments = Vec::new();
        let location = self.previous().location.clone();

        if !self.check(TokenKind::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightParen, "Expected ')' after arguments")?;

        Ok(Expression::Call(CallExpr {
            function: Box::new(callee),
            arguments,
            location,
        }))
    }

    /// Parse primary expressions (literals, identifiers, groupings)
    fn primary(&mut self) -> Result<Expression> {
        if self.match_token(TokenKind::True) {
            Ok(Expression::Literal(Literal::Bool(true)))
        } else if self.match_token(TokenKind::False) {
            Ok(Expression::Literal(Literal::Bool(false)))
        } else if let TokenKind::IntegerLiteral(value) = self.peek().kind {
            self.advance();
            Ok(Expression::Literal(Literal::Int(value)))
        } else if let TokenKind::StringLiteral(value) = &self.peek().kind {
            let value = value.clone();
            self.advance();
            Ok(Expression::Literal(Literal::String(value)))
        } else if let TokenKind::CharLiteral(value) = self.peek().kind {
            self.advance();
            Ok(Expression::Literal(Literal::Char(value)))
        } else if self.match_token(TokenKind::LeftParen) {
            if self.match_token(TokenKind::RightParen) {
                Ok(Expression::Literal(Literal::Unit))
            } else {
                let expr = self.expression()?;
                self.consume(TokenKind::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
        } else if self.match_token(TokenKind::If) {
            self.if_expression()
        } else if self.match_token(TokenKind::Case) {
            self.case_expression()
        } else if self.match_token(TokenKind::Do) {
            self.do_expression()
        } else if self.match_token(TokenKind::Region) {
            // Handle region() function call - creates a new region
            let location = self.previous().location.clone();
            self.consume(TokenKind::LeftParen, "Expected '(' after 'region'")?;
            self.consume(TokenKind::RightParen, "Expected ')' after 'region('")?;
            // This will be handled by the runtime system
            Ok(Expression::Literal(Literal::Int(0))) // Placeholder for region handle
        } else if self.match_token(TokenKind::Spawn) {
            // Handle spawn(initial_state, behavior) expression
            self.parse_spawn()
        } else if self.match_token(TokenKind::Send) {
            // Handle send(actor, message) expression
            self.parse_send()
        } else if self.match_token(TokenKind::Recv) {
            // Handle recv() expression
            self.parse_recv()
        } else if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();

            // Check for special built-in operations
            if name == "alloc_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_alloc_ref()
            } else if name == "read_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_read_ref()
            } else if name == "write_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_write_ref()
            } else {
                // Regular identifier
                Ok(Expression::Identifier(name))
            }
        } else if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(Expression::Identifier(name))
        } else {
            parse_error(
                self.peek().location.clone(),
                "Expected expression".to_string(),
            )
        }
    }

    /// Parse alloc_ref(region, initial_value) expression
    fn parse_alloc_ref(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let region = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after region in alloc_ref")?;
        let initial_value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after alloc_ref arguments")?;

        Ok(Expression::AllocRef(AllocRefExpr {
            region,
            initial_value,
            location,
        }))
    }

    /// Parse read_ref(reference) expression
    fn parse_read_ref(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let reference = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after read_ref argument")?;

        Ok(Expression::ReadRef(ReadRefExpr {
            reference,
            location,
        }))
    }

    /// Parse write_ref(reference, value) expression
    fn parse_write_ref(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let reference = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after reference in write_ref")?;
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after write_ref arguments")?;

        Ok(Expression::WriteRef(WriteRefExpr {
            reference,
            value,
            location,
        }))
    }

    /// Parse spawn(initial_state, behavior) expression
    fn parse_spawn(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::LeftParen, "Expected '(' after 'spawn'")?;
        let initial_state = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after initial_state in spawn")?;
        let behavior = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after spawn arguments")?;

        Ok(Expression::Spawn(SpawnExpr {
            initial_state,
            behavior,
            location,
        }))
    }

    /// Parse send(actor, message) expression
    fn parse_send(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::LeftParen, "Expected '(' after 'send'")?;
        let actor = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after actor in send")?;
        let message = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after send arguments")?;

        Ok(Expression::Send(SendExpr {
            actor,
            message,
            location,
        }))
    }

    /// Parse recv() expression
    fn parse_recv(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::LeftParen, "Expected '(' after 'recv'")?;
        self.consume(TokenKind::RightParen, "Expected ')' after recv")?;

        Ok(Expression::Recv(RecvExpr {
            location,
        }))
    }

    /// Parse struct declaration: struct Name<T, U> { field: Type, ... }
    fn struct_declaration(&mut self) -> Result<StructDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected struct name")?;

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

        self.consume(TokenKind::LeftBrace, "Expected '{' after struct name")?;

        let mut fields = Vec::new();
        while !self.check(TokenKind::RightBrace) {
            let field_name = self.consume_identifier("Expected field name")?;
            self.consume(TokenKind::Colon, "Expected ':' after field name")?;
            let field_type = self.parse_type()?;

            fields.push(StructField {
                name: field_name,
                ty: field_type,
                location: self.previous().location.clone(),
            });

            if !self.match_token(TokenKind::Comma) {
                // No comma, this should be the last field
                break;
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after struct fields")?;

        Ok(StructDecl {
            name,
            type_params,
            fields,
            location,
        })
    }

    /// Parse enum declaration: enum Name<T, U> { Variant1, Variant2(Type), Variant3 { field: Type } }
    fn enum_declaration(&mut self) -> Result<EnumDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected enum name")?;

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

        self.consume(TokenKind::LeftBrace, "Expected '{' after enum name")?;

        let mut variants = Vec::new();
        if !self.check(TokenKind::RightBrace) {
            loop {
                let variant_name = self.consume_identifier("Expected variant name")?;
                let variant_location = self.previous().location.clone();

                let variant = if self.match_token(TokenKind::LeftParen) {
                    // Tuple variant: Variant(Type, Type)
                    let mut fields = Vec::new();
                    if !self.check(TokenKind::RightParen) {
                        loop {
                            fields.push(self.parse_type()?);
                            if !self.match_token(TokenKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RightParen, "Expected ')' after tuple fields")?;
                    EnumVariant::Tuple { name: variant_name, fields, location: variant_location }
                } else if self.match_token(TokenKind::LeftBrace) {
                    // Struct variant: Variant { field: Type, ... }
                    let mut fields = Vec::new();
                    if !self.check(TokenKind::RightBrace) {
                        loop {
                            let field_name = self.consume_identifier("Expected field name")?;
                            self.consume(TokenKind::Colon, "Expected ':' after field name")?;
                            let field_type = self.parse_type()?;
                            self.consume(TokenKind::Comma, "Expected ',' after field")?;

                            fields.push(StructField {
                                name: field_name,
                                ty: field_type,
                                location: self.previous().location.clone(),
                            });

                            if self.check(TokenKind::RightBrace) {
                                break;
                            }
                        }
                    }
                    self.consume(TokenKind::RightBrace, "Expected '}' after struct fields")?;
                    EnumVariant::Struct { name: variant_name, fields, location: variant_location }
                } else {
                    // Unit variant: Variant
                    EnumVariant::Unit { name: variant_name, location: variant_location }
                };

                variants.push(variant);

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after enum variants")?;

        Ok(EnumDecl {
            name,
            type_params,
            variants,
            location,
        })
    }

    /// Parse trait declaration: trait Name<T, U> { fn method(&self, param: Type) -> ReturnType; }
    fn trait_declaration(&mut self) -> Result<TraitDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected trait name")?;

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

        self.consume(TokenKind::LeftBrace, "Expected '{' after trait name")?;

        let mut methods = Vec::new();
        if !self.check(TokenKind::RightBrace) {
            loop {
                // For now, only support method declarations (not implementations)
                self.consume(TokenKind::Fn, "Expected 'fn' for trait method")?;
                let method_name = self.consume_identifier("Expected method name")?;
                let method_location = self.previous().location.clone();

                self.consume(TokenKind::LeftParen, "Expected '(' after method name")?;

                let mut params = Vec::new();
                if !self.check(TokenKind::RightParen) {
                    loop {
                        let param_name = self.consume_identifier("Expected parameter name")?;

                        // Check if this is a self parameter (no type annotation)
                        let param_type = if self.match_token(TokenKind::Colon) {
                            self.parse_type()?
                        } else {
                            // For self parameters, use a placeholder type
                            // In a real implementation, this would be resolved to the implementing type
                            Type::Named("Self".to_string())
                        };

                        params.push(Parameter {
                            name: param_name,
                            type_: param_type,
                            location: self.previous().location.clone(),
                        });

                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.consume(TokenKind::RightParen, "Expected ')' after parameters")?;

                let return_type = if self.match_token(TokenKind::RightArrow) {
                    Some(self.parse_type()?)
                } else {
                    None
                };

                self.consume(TokenKind::Semicolon, "Expected ';' after trait method")?;

                methods.push(TraitMethod {
                    name: method_name,
                    params,
                    return_type,
                    location: method_location,
                });

                if self.check(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after trait methods")?;

        Ok(TraitDecl {
            name,
            type_params,
            methods,
            location,
        })
    }

    /// Parse impl declaration: impl<T> Trait for Type { ... } or impl Type { ... }
    fn impl_declaration(&mut self) -> Result<ImplDecl> {
        let location = self.previous().location.clone();

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

        // Check if this is a trait implementation
        let trait_name = if let TokenKind::Identifier(name) = &self.peek().kind {
            let name_clone = name.clone();
            self.advance();
            if self.match_token(TokenKind::For) {
                Some(name_clone)
            } else {
                // This was actually the type name, not a trait name
                return parse_error(location, "Expected 'for' after trait name in impl".to_string());
            }
        } else {
            None // Inherent impl
        };

        // Parse the type being implemented for
        let for_type = self.parse_type()?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after impl type")?;

        let mut methods = Vec::new();
        if !self.check(TokenKind::RightBrace) {
            loop {
                let method = self.function_declaration()?;
                methods.push(method);

                if self.check(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after impl methods")?;

        Ok(ImplDecl {
            trait_name,
            type_params,
            for_type,
            methods,
            location,
        })
    }

    /// Parse type alias declaration: type Name<T, U> = Type;
    fn type_alias_declaration(&mut self) -> Result<TypeAliasDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected type alias name")?;

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

        self.consume(TokenKind::Equal, "Expected '=' after type alias name")?;
        let aliased_type = self.parse_type()?;
        self.consume(TokenKind::Semicolon, "Expected ';' after type alias")?;

        Ok(TypeAliasDecl {
            name,
            type_params,
            aliased_type,
            location,
        })
    }

    /// Parse type parameters: <T, U, V>
    fn parse_type_parameters(&mut self) -> Result<Vec<String>> {
        let mut params = Vec::new();

        if !self.check(TokenKind::Greater) {
            loop {
                let param = self.consume_identifier("Expected type parameter name")?;
                params.push(param);

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::Greater, "Expected '>' after type parameters")?;
        Ok(params)
    }

    /// Parse if expression
    fn if_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let condition = Box::new(self.expression()?);
        self.consume(TokenKind::LeftBrace, "Expected '{' after if condition")?;
        let then_branch = Box::new(self.expression()?);
        self.consume(TokenKind::RightBrace, "Expected '}' after then branch")?;
        self.consume(TokenKind::Else, "Expected 'else' after then branch")?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after else")?;
        let else_branch = Box::new(self.expression()?);
        self.consume(TokenKind::RightBrace, "Expected '}' after else branch")?;

        Ok(Expression::If(IfExpr {
            condition,
            then_branch,
            else_branch,
            location,
        }))
    }

    /// Parse case expression (simplified for now)
    fn case_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let scrutinee = Box::new(self.expression()?);
        self.consume(TokenKind::Of, "Expected 'of' after case expression")?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after 'of'")?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            let pattern = self.pattern()?;
            self.consume(TokenKind::RightArrow, "Expected '->' after pattern")?;
            let body = self.expression()?;
            self.consume(TokenKind::Semicolon, "Expected ';' after case branch")?;

            branches.push(CaseBranch {
                pattern,
                body: Box::new(body),
                location: self.previous().location.clone(),
            });
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after case branches")?;

        Ok(Expression::Case(CaseExpr {
            scrutinee,
            branches,
            location,
        }))
    }

    /// Parse do expression
    fn do_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let mut statements = Vec::new();

        while !self.check(TokenKind::End) && !self.is_at_end() {
            if self.match_token(TokenKind::Let) {
                let pattern = self.pattern()?;
                self.consume(TokenKind::LeftArrow, "Expected '<-' after pattern")?;
                let expr = self.expression()?;
                self.consume(TokenKind::Semicolon, "Expected ';' after binding")?;
                statements.push(Statement::Bind {
                    pattern,
                    expr: Box::new(expr),
                });
            } else {
                let expr = self.expression()?;
                self.consume(TokenKind::Semicolon, "Expected ';' after expression")?;
                statements.push(Statement::Expr(Box::new(expr)));
            }
        }

        self.consume(TokenKind::End, "Expected 'end' after do block")?;

        Ok(Expression::Do(DoExpr {
            statements,
            location,
        }))
    }

    /// Parse pattern
    fn pattern(&mut self) -> Result<Pattern> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(Pattern::Identifier(name))
        } else if self.match_token(TokenKind::Underscore) {
            Ok(Pattern::Wildcard)
        } else if self.match_token(TokenKind::LeftParen) {
            let mut patterns = Vec::new();
            if !self.check(TokenKind::RightParen) {
                loop {
                    patterns.push(self.pattern()?);
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RightParen, "Expected ')' after tuple pattern")?;
            Ok(Pattern::Tuple(patterns))
        } else {
            parse_error(
                self.peek().location.clone(),
                "Expected pattern".to_string(),
            )
        }
    }

    // Helper methods
    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek().kind == kind
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || self.peek().kind == TokenKind::EOF
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<&Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            parse_error(self.peek().location.clone(), message.to_string())
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String> {
        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            parse_error(self.peek().location.clone(), message.to_string())
        }
    }

    fn consume_integer(&mut self, message: &str) -> Result<i64> {
        if let TokenKind::IntegerLiteral(value) = self.peek().kind {
            self.advance();
            Ok(value)
        } else {
            parse_error(self.peek().location.clone(), message.to_string())
        }
    }
}
