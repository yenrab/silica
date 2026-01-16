use crate::ast::*;
use crate::errors::{Result, SourceLocation, parse_error, parse_error_with_metadata, ErrorMetadataBuilder, ErrorSeverity};
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

    /// Validate and fix token position if corrupted
    fn validate_token_position(&self, token: &Token) -> SourceLocation {
        if token.location.line > 10000 || token.location.line < 1 {
            // Position is corrupted, create a synthetic location
            SourceLocation::new(
                token.location.file.clone(),
                1, // Default to line 1
                1, // Default to column 1
                token.location.offset,
            )
        } else {
            token.location.clone()
        }
    }

    /// Parse a top-level declaration
    fn declaration(&mut self) -> Result<Declaration> {
        let current_token = self.peek();
        let valid_location = self.validate_token_position(current_token);

        if self.match_token(TokenKind::Fn) {
            let result = self.function_declaration().map(Declaration::Function);
            result
        } else if self.match_token(TokenKind::Type) {
            // For now, only support type aliases
            self.type_alias_declaration().map(Declaration::TypeAlias)
        } else if self.match_token(TokenKind::Effect) {
            self.effect_declaration().map(Declaration::Effect)
        } else if self.match_token(TokenKind::Use) {
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
            // eprintln!("DEBUG PARSER: Matched impl token, calling impl_declaration");
            let result = self.impl_declaration().map(Declaration::Impl);
            // eprintln!("DEBUG PARSER: impl_declaration result: {:?}", result.is_ok());
            result
        } else {
            let location = self.peek().location.clone();
            let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§3.2"), None)
                .build();
            parse_error_with_metadata(
                location,
                "Expected declaration (fn, type, effect, module, import, export, struct, enum, trait, or impl)".to_string(),
                metadata,
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
        let body = self.parse_statements()?;
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

        // Check for tuple pattern parameter
        if self.check(TokenKind::LeftParen) {
            let pattern = self.pattern()?;
            self.consume(TokenKind::Colon, "Expected ':' after parameter pattern")?;
            let type_ = self.parse_type()?;
            return Ok(Parameter {
                name: "_".to_string(), // Placeholder name for pattern parameters
                type_,
                location,
                pattern: Some(pattern),
            });
        }

        // Allow 'self' as a special parameter name
        let name = if self.match_token(TokenKind::Self_) {
            "self".to_string()
        } else {
            self.consume_identifier("Expected parameter name")?
        };

        // Check if this is a &self parameter (special marker from consume_identifier)
        if name == "&self" {
            return Ok(Parameter {
                name: "self".to_string(),
                type_: Type::Named("&Self".to_string()), // Placeholder for reference to Self
                location,
                pattern: None,
            });
        }

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
            pattern: None,
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


    /// Parse module path: ident::ident::...
    fn module_path(&mut self) -> Result<Vec<String>> {
        let mut path = Vec::new();
        path.push(self.consume_identifier("Expected module name")?);

        while self.match_token(TokenKind::DoubleColon) {
            path.push(self.consume_identifier("Expected module name after '::'")?);
        }

        Ok(path)
    }

    /// Parse import declaration: use module1, module2, module3;
    fn import_declaration(&mut self) -> Result<ImportDecl> {
        let location = self.previous().location.clone();
        let mut modules = Vec::new();

        // Parse first module name
        modules.push(self.consume_identifier("Expected module name after 'use'")?);

        // Parse additional module names separated by commas
        while self.match_token(TokenKind::Comma) {
            modules.push(self.consume_identifier("Expected module name after ','")?);
        }

        self.consume(TokenKind::Semicolon, "Expected ';' after import")?;

        Ok(ImportDecl {
            modules,
            location,
        })
    }

    /// Parse export declaration: export item/arity, item/arity;
    fn export_declaration(&mut self) -> Result<ExportDecl> {
        let location = self.previous().location.clone();
        let mut items = Vec::new();

        // Parse first export item
        items.push(self.parse_export_item()?);

        // Parse additional items separated by commas
        while self.match_token(TokenKind::Comma) {
            items.push(self.parse_export_item()?);
        }

        self.consume(TokenKind::Semicolon, "Expected ';' after export")?;

        Ok(ExportDecl {
            items,
            location,
        })
    }

    /// Parse a single export item: name/arity
    fn parse_export_item(&mut self) -> Result<ExportItem> {
        let location = self.peek().location.clone();
        let name = self.consume_identifier("Expected identifier in export")?;
        self.consume(TokenKind::Slash, "Expected '/' after export name")?;
        let arity = self.consume_integer("Expected arity number after '/'")?;
        let arity = arity as u32;

        Ok(ExportItem {
            name,
            arity,
            location,
        })
    }

    /// Parse a type expression
    fn parse_type(&mut self) -> Result<Type> {
        // Built-in types (check these first to avoid conflicts)
        // Check numeric types in order to avoid partial matches (int64 before int)
        if self.match_token(TokenKind::Int64) {
            return Ok(Type::Int64);
        } else if self.match_token(TokenKind::Int32) {
            return Ok(Type::Int32);
        } else if self.match_token(TokenKind::Int16) {
            return Ok(Type::Int16);
        } else if self.match_token(TokenKind::Int8) {
            return Ok(Type::Int8);
        } else if self.match_token(TokenKind::Float64) {
            return Ok(Type::Float64);
        } else if self.match_token(TokenKind::Float32) {
            return Ok(Type::Float32);
        } else if self.match_token(TokenKind::Float16) {
            return Ok(Type::Float16);
        } else if self.match_token(TokenKind::Bool) {
            return Ok(Type::Bool);
        } else if self.match_token(TokenKind::Char) {
            return Ok(Type::Char);
        } else if self.match_token(TokenKind::Unit) {
            return Ok(Type::Unit);
        } else if self.match_token(TokenKind::String) {
            return Ok(Type::String);
        }

        // Reference type: &Type
        if self.match_token(TokenKind::Ampersand) {
            let element_type = self.parse_type()?;
            // For now, create a placeholder reference type
            // In a full implementation, this would create a proper Reference type
            let type_str = match &element_type {
                Type::Named(name) => name.clone(),
                Type::Unit => "unit".to_string(),
                Type::Bool => "bool".to_string(),
                Type::Int8 => "int8".to_string(),
                Type::Int16 => "int16".to_string(),
                Type::Int32 => "int32".to_string(),
                Type::Int64 => "int64".to_string(),
                Type::Float16 => "float16".to_string(),
                Type::Float32 => "float32".to_string(),
                Type::Char => "char".to_string(),
                Type::String => "string".to_string(),
                _ => "unknown".to_string(), // Placeholder for complex types
            };
            return Ok(Type::Named(format!("&{}", type_str)));
        }

        // Polymorphic function type: fn<T, U>(param_types) -> return_type
        if self.match_token(TokenKind::Fn) {
            // Parse type parameters
            let type_params = if self.match_token(TokenKind::Less) {
                let mut params = Vec::new();
                loop {
                    let param_name = self.consume_identifier("Expected type parameter name")?;
                    params.push(param_name);
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
                self.consume(TokenKind::Greater, "Expected '>' after type parameters")?;
                params
            } else {
                Vec::new()
            };

            // Parse parameter types
            self.consume(TokenKind::LeftParen, "Expected '(' after 'fn'")?;
            let mut parameters = Vec::new();
            if !self.check(TokenKind::RightParen) {
                loop {
                    let param_type = self.parse_type()?;
                    parameters.push(param_type);
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::RightParen, "Expected ')' after parameters")?;

            // Parse return type
            self.consume(TokenKind::RightArrow, "Expected '->' after parameters")?;
            let return_type = self.parse_type()?;

            // Temporary fix: treat all fn<T>(...) as regular functions
            // This handles the case where T is already in scope
            return Ok(Type::Function {
                parameters,
                return_type: Box::new(return_type),
            });
        }


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
            // Actor reference type - primitive type (like int, bool)
            Ok(Type::ActorRef)
        } else {
            // User-defined type name or type operator
            let name = self.consume_identifier("Expected type name")?;

            // Check if this is a type operator with arguments
            if self.match_token(TokenKind::Less) {
                // Type operator: Name<Arg1, Arg2, ...>
                let mut args = Vec::new();
                if !self.check(TokenKind::Greater) {
                    loop {
                        args.push(self.parse_type()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::Greater, "Expected '>' after type arguments")?;
                Ok(Type::TypeOperator { name, args })
            } else {
                // Check for sum types: Type | Type | Type
                let mut types = vec![Type::Named(name)];
                while self.match_token(TokenKind::Pipe) {
                    types.push(self.parse_type()?);
                }

                if types.len() == 1 {
                    Ok(types.into_iter().next().unwrap())
                } else {
                    Ok(Type::Sum(types))
                }
            }
        }
    }

    /// Parse type with optional type parameters in scope
    fn parse_type_with_params(&mut self, _type_params: &[String]) -> Result<Type> {
        self.parse_type()
    }

    /// Parse memory space
    fn memory_space(&mut self) -> Result<MemorySpace> {
        if self.match_token(TokenKind::Normal) {
            Ok(MemorySpace::Normal)
        } else if self.match_token(TokenKind::Atomic) {
            Ok(MemorySpace::Atomic)
        } else {
            let location = self.peek().location.clone();
            let metadata = self.build_parse_error_metadata("E1006", &location, Some("spec:§10"), None)
                .suggestion("Use 'normal' for normal memory space".to_string())
                .suggestion("Use 'atomic' for atomic memory space".to_string())
                .build();
            parse_error_with_metadata(
                location,
                "Expected 'normal' or 'atomic' memory space".to_string(),
                metadata,
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
        let current_token = self.peek();
        eprintln!("DEBUG PARSER: expression() called at {}:{} (token: {:?} '{}')", 
                 current_token.location.line, current_token.location.column, 
                 current_token.kind, current_token.lexeme);
        let result = self.assignment();
        if let Ok(ref expr) = result {
            eprintln!("DEBUG PARSER: expression() succeeded, returning {:?}", std::mem::discriminant(expr));
        } else if let Err(ref e) = result {
            eprintln!("DEBUG PARSER: expression() failed: {:?}", e);
        }
        result
    }

    /// Parse assignment expression (lowest precedence)
    fn assignment(&mut self) -> Result<Expression> {
        // Fall back to regular expression parsing
        let expr = self.or()?;
        Ok(expr)
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
        let current_token = self.peek();
        eprintln!("DEBUG PARSER: term() called at {}:{} (token: {:?} '{}')", 
                 current_token.location.line, current_token.location.column, 
                 current_token.kind, current_token.lexeme);
        let mut expr = self.factor()?;

        while self.match_token(TokenKind::Plus) || self.match_token(TokenKind::Minus) {
            eprintln!("DEBUG PARSER: term() matched operator {:?}", self.previous().kind);
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
        if self.match_token(TokenKind::Bang) || self.match_token(TokenKind::Not) || self.match_token(TokenKind::Minus) {
            let operator = if self.previous().kind == TokenKind::Bang || self.previous().kind == TokenKind::Not {
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

    /// Parse function calls, field access, type casting, and primary expressions
    fn call(&mut self) -> Result<Expression> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(TokenKind::Dot) {
                expr = self.finish_field_access(expr)?;
            } else if self.match_token(TokenKind::As) {
                // Type casting: expr as Type
                let as_location = self.previous().location.clone();
                let target_type = self.parse_type()?;
                expr = Expression::AsType(AsTypeExpr {
                    expression: Box::new(expr),
                    target_type,
                    location: as_location,
                });
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


        // Parse arguments
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


    /// Finish parsing field access: object.field
    fn finish_field_access(&mut self, object: Expression) -> Result<Expression> {
        let location = self.previous().location.clone();
        let field = self.consume_identifier("Expected field name after '.'")?;

        Ok(Expression::FieldAccess(FieldAccessExpr {
            object: Box::new(object),
            field,
            location,
        }))
    }

    /// Finish parsing struct literal: TypeName { field: value, ... }
    fn finish_struct_literal(&mut self, type_expr: Expression) -> Result<Expression> {
        let location = self.previous().location.clone();

        // Extract type name from the expression (should be an identifier)
        let type_name = match type_expr {
            Expression::Identifier(name) => name,
            _ => {
                let metadata = self.build_parse_error_metadata("E1003", &location, Some("spec:§3"), None)
                    .build();
                return parse_error_with_metadata(
                    location.clone(),
                    "Expected type name before struct literal".to_string(),
                    metadata,
                );
            }
        };

        let mut fields = Vec::new();

        // Parse struct literal fields: { field: value, field2: value2 }
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            let field_name = self.consume_identifier("Expected field name")?;
            self.consume(TokenKind::Colon, "Expected ':' after field name")?;
            let field_value = self.expression()?;

            fields.push((field_name, field_value));

            if !self.match_token(TokenKind::Comma) && !self.check(TokenKind::RightBrace) {
                let location = self.peek().location.clone();
                let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§3"), None)
                    .build();
                return parse_error_with_metadata(
                    location,
                    "Expected ',' or '}' after field".to_string(),
                    metadata,
                );
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after struct literal")?;

        Ok(Expression::StructLiteral(StructLiteralExpr {
            type_name,
            fields,
            location,
        }))
    }

    /// Parse primary expressions (literals, identifiers, groupings)
    fn primary(&mut self) -> Result<Expression> {
        let current_token = self.peek();
        eprintln!("DEBUG PARSER: primary() called at {}:{} (token: {:?} '{}')", 
                 current_token.location.line, current_token.location.column, 
                 current_token.kind, current_token.lexeme);
        // Handle core affinity keywords
        if self.match_token(TokenKind::EfficiencyCores) {
            return Ok(Expression::Identifier("efficiency_cores".to_string()));
        } else if self.match_token(TokenKind::PerformanceCores) {
            return Ok(Expression::Identifier("performance_cores".to_string()));
        }

        if self.match_token(TokenKind::True) {
            Ok(Expression::Literal(Literal::Bool(true)))
        } else if self.match_token(TokenKind::False) {
            Ok(Expression::Literal(Literal::Bool(false)))
        } else if let TokenKind::IntegerLiteral(value) = self.peek().kind.clone() {
            self.advance();
            Ok(Expression::Literal(Literal::Int(value)))
        } else if let TokenKind::FloatLiteral(value) = self.peek().kind.clone() {
            self.advance();
            Ok(Expression::Literal(Literal::Float(value)))
        } else if let TokenKind::StringLiteral(value) = self.peek().kind.clone() {
            self.advance();
            Ok(Expression::Literal(Literal::String(value)))
        } else if let TokenKind::CharLiteral(value) = self.peek().kind.clone() {
            self.advance();
            Ok(Expression::Literal(Literal::Char(value)))
        } else if self.match_token(TokenKind::LeftParen) {
            if self.match_token(TokenKind::RightParen) {
                Ok(Expression::Literal(Literal::Unit))
            } else {
                let first_expr = self.expression()?;
                if self.match_token(TokenKind::Comma) {
                    // Tuple expression: (expr1, expr2, ...)
                    let mut expressions = vec![first_expr];
                    loop {
                        expressions.push(self.expression()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.consume(TokenKind::RightParen, "Expected ')' after tuple expression")?;
                    Ok(Expression::Tuple(expressions))
                } else {
                    // Parenthesized expression: (expr)
                self.consume(TokenKind::RightParen, "Expected ')' after expression")?;
                    Ok(first_expr)
                }
            }
        } else if self.match_token(TokenKind::Case) {
            self.case_expression()
        } else if self.match_token(TokenKind::Do) {
            self.do_expression()
        } else if self.match_token(TokenKind::Fn) {
            self.function_literal()
        } else if self.match_token(TokenKind::Region) {
            // Handle region() built-in operation - creates a new region
            let location = self.previous().location.clone();
            self.consume(TokenKind::LeftParen, "Expected '(' after 'region'")?;
            self.consume(TokenKind::RightParen, "Expected ')' after 'region('")?;
            Ok(Expression::Region(RegionExpr {
                space: MemorySpace::Normal,
                location,
            }))
        } else if self.match_token(TokenKind::Self_) {
            // Handle 'self' as an identifier in expressions
            Ok(Expression::Identifier("self".to_string()))
        } else if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            let start_location = self.peek().location.clone();
            
            // Check if this is the forbidden 'let' keyword
            if name == "let" {
                let metadata = self.build_parse_error_metadata("E1007", &start_location, Some("spec:§3.3"), None)
                    .suggestion("Remove 'let' keyword. In Silica, bindings use pattern: Type <- value syntax without 'let'.".to_string())
                    .suggestion_with_example("Use:".to_string(), "x: int64 <- 42;".to_string())
                    .build();
                return parse_error_with_metadata(
                    start_location,
                    "'let' is not a keyword in Silica. Use pattern: Type <- value syntax instead.".to_string(),
                    metadata,
                );
            }
            
            eprintln!("DEBUG PARSER: primary() found identifier '{}' at {}:{}", name, start_location.line, start_location.column);
            self.advance(); // consume the identifier

            // Check for constructor syntax: TypeName::Constructor
            if self.match_token(TokenKind::DoubleColon) {
                self.parse_constructor_call(name, start_location)
            }
            // Check for special built-in operations
            else if name == "region" && self.match_token(TokenKind::LeftParen) {
                self.parse_region()
            } else if name == "alloc_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_alloc_ref()
            } else if name == "read_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_read_ref()
            } else if name == "core_id" && self.match_token(TokenKind::LeftParen) {
                self.parse_core_id()
            } else if name == "any_core" {
                Ok(Expression::Identifier("any_core".to_string()))
            } else if name == "write_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_write_ref()
            } else if name == "exec_command" && self.match_token(TokenKind::LeftParen) {
                self.parse_exec_command()
            } else if name == "read_file" && self.match_token(TokenKind::LeftParen) {
                self.parse_read_file()
            } else if name == "write_file" && self.match_token(TokenKind::LeftParen) {
                self.parse_write_file()
            } else if name == "print" && self.match_token(TokenKind::LeftParen) {
                self.parse_print()
            } else if name == "println" && self.match_token(TokenKind::LeftParen) {
                self.parse_println()
            } else if name == "print_int64" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_int64()
            } else if name == "print_int32" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_int32()
            } else if name == "print_int16" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_int16()
            } else if name == "print_int8" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_int8()
            } else if name == "print_bool" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_bool()
            } else if name == "print_char" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_char()
            } else if name == "print_float16" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_float16()
            } else if name == "print_float32" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_float32()
            } else if name == "print_float64" && self.match_token(TokenKind::LeftParen) {
                self.parse_print_float64()
            } else if name == "get_cpu_topology_info" && self.match_token(TokenKind::LeftParen) {
                self.parse_get_cpu_topology_info()
            } else if name == "read_lines" && self.match_token(TokenKind::LeftParen) {
                self.parse_read_lines()
            } else if name == "append_file" && self.match_token(TokenKind::LeftParen) {
                self.parse_append_file()
            } else if name == "file_exists" && self.match_token(TokenKind::LeftParen) {
                self.parse_file_exists()
            } else if name == "delete_file" && self.match_token(TokenKind::LeftParen) {
                self.parse_delete_file()
            } else if name == "get_file_size" && self.match_token(TokenKind::LeftParen) {
                self.parse_get_file_size()
            } else if name == "create_directory" && self.match_token(TokenKind::LeftParen) {
                self.parse_create_directory()
            } else if name == "remove_directory" && self.match_token(TokenKind::LeftParen) {
                self.parse_remove_directory()
            } else if name == "list_directory" && self.match_token(TokenKind::LeftParen) {
                self.parse_list_directory()
            } else if name == "len" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_len()
            } else if name == "len_chars" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_len_chars()
            } else if name == "concat" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_concat()
            } else if name == "substring" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_substring()
            } else if name == "substring_until_char" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_substring_until_char()
            } else if name == "starts_with" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_starts_with()
            } else if name == "ends_with" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_ends_with()
            } else if name == "contains" && self.match_token(TokenKind::LeftParen) {
                self.parse_string_contains()
            } else if self.match_token(TokenKind::LeftBrace) {
                // Parse struct literal: TypeName { field: value, ... }
                let type_expr = Expression::Identifier(name);
                self.finish_struct_literal(type_expr)
            } else {
                // Just an identifier - < and ( will be handled by call() logic
                Ok(Expression::Identifier(name))
            }
        } else if self.match_token(TokenKind::Spawn) {
            // Handle spawn(initial_state, behavior) expression
            self.parse_spawn()
        } else if self.match_token(TokenKind::Send) {
            // Handle send(actor, message) expression
            self.parse_send()
        } else if self.match_token(TokenKind::Recv) {
            // Handle recv() expression
            self.parse_recv()
        } else if self.match_token(TokenKind::Cast) {
            // Handle cast(actor, message) expression
            self.parse_cast()
        } else {
            let location = self.peek().location.clone();
            let metadata = self.build_parse_error_metadata("E1005", &location, Some("spec:§3.3"), None)
                .build();
            parse_error_with_metadata(
                location,
                "Expected expression".to_string(),
                metadata,
            )
        }
    }

    /// Parse region() expression
    fn parse_region(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::RightParen, "Expected ')' after region")?;

        Ok(Expression::Region(RegionExpr {
            space: MemorySpace::Normal, // Default to normal memory space
            location,
        }))
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

    /// Parse read_file(path) expression
    fn parse_read_file(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after read_file argument")?;

        Ok(Expression::ReadFile(ReadFileExpr {
            path,
            location,
        }))
    }

    /// Parse write_file(path, content) expression
    fn parse_write_file(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after path in write_file")?;
        let content = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after write_file arguments")?;

        Ok(Expression::WriteFile(WriteFileExpr {
            path,
            content,
            location,
        }))
    }

    /// Parse print(value) expression
    fn parse_print(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print argument")?;

        Ok(Expression::Print(PrintExpr {
            value,
            location,
        }))
    }

    /// Parse println(value) expression
    fn parse_println(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after println argument")?;

        Ok(Expression::PrintLn(PrintLnExpr {
            value,
            location,
        }))
    }

    /// Parse print_int64(value) expression
    fn parse_print_int64(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_int64 argument")?;

        Ok(Expression::PrintInt64(PrintInt64Expr {
            value,
            location,
        }))
    }

    /// Parse print_int8(value) expression
    fn parse_print_int8(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_int8 argument")?;

        Ok(Expression::PrintInt8(PrintInt8Expr {
            value,
            location,
        }))
    }

    /// Parse print_int16(value) expression
    fn parse_print_int16(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_int16 argument")?;

        Ok(Expression::PrintInt16(PrintInt16Expr {
            value,
            location,
        }))
    }

    /// Parse print_int32(value) expression
    fn parse_print_int32(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_int32 argument")?;

        Ok(Expression::PrintInt32(PrintInt32Expr {
            value,
            location,
        }))
    }

    /// Parse print_bool(value) expression
    fn parse_print_bool(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_bool argument")?;

        Ok(Expression::PrintBool(PrintBoolExpr {
            value,
            location,
        }))
    }

    /// Parse print_char(value) expression
    fn parse_print_char(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_char argument")?;

        Ok(Expression::PrintChar(PrintCharExpr {
            value,
            location,
        }))
    }

    /// Parse print_float16(value) expression
    fn parse_print_float16(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_float16 argument")?;

        Ok(Expression::PrintFloat16(PrintFloat16Expr {
            value,
            location,
        }))
    }

    /// Parse print_float32(value) expression
    fn parse_print_float32(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_float32 argument")?;

        Ok(Expression::PrintFloat32(PrintFloat32Expr {
            value,
            location,
        }))
    }

    /// Parse print_float64(value) expression
    fn parse_print_float64(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let value = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after print_float64 argument")?;

        Ok(Expression::PrintFloat64(PrintFloat64Expr {
            value,
            location,
        }))
    }

    /// Parse get_cpu_topology_info() expression
    fn parse_get_cpu_topology_info(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::RightParen, "Expected ')' after get_cpu_topology_info")?;

        Ok(Expression::GetCpuTopologyInfo(GetCpuTopologyInfoExpr {
            location,
        }))
    }

    /// Parse read_lines(path) expression
    fn parse_read_lines(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after read_lines argument")?;

        Ok(Expression::ReadLines(ReadLinesExpr {
            path,
            location,
        }))
    }

    /// Parse append_file(path, content) expression
    fn parse_append_file(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after path in append_file")?;
        let content = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after append_file arguments")?;

        Ok(Expression::AppendFile(AppendFileExpr {
            path,
            content,
            location,
        }))
    }

    /// Parse file_exists(path) expression
    fn parse_file_exists(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after file_exists argument")?;

        Ok(Expression::FileExists(FileExistsExpr {
            path,
            location,
        }))
    }

    /// Parse delete_file(path) expression
    fn parse_delete_file(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after delete_file argument")?;

        Ok(Expression::DeleteFile(DeleteFileExpr {
            path,
            location,
        }))
    }

    /// Parse get_file_size(path) expression
    fn parse_get_file_size(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after get_file_size argument")?;

        Ok(Expression::GetFileSize(GetFileSizeExpr {
            path,
            location,
        }))
    }

    /// Parse create_directory(path) expression
    fn parse_create_directory(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after create_directory argument")?;

        Ok(Expression::CreateDirectory(CreateDirectoryExpr {
            path,
            location,
        }))
    }

    /// Parse remove_directory(path) expression
    fn parse_remove_directory(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after remove_directory argument")?;

        Ok(Expression::RemoveDirectory(RemoveDirectoryExpr {
            path,
            location,
        }))
    }

    /// Parse list_directory(path) expression
    fn parse_list_directory(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let path = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after list_directory argument")?;

        Ok(Expression::ListDirectory(ListDirectoryExpr {
            path,
            location,
        }))
    }

    /// Parse string length expression: len(s) - returns byte count
    fn parse_string_len(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after len argument")?;

        Ok(Expression::StringLen(StringLenExpr {
            string,
            location,
        }))
    }

    /// Parse string character length expression: len_chars(s) - returns character count
    fn parse_string_len_chars(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after len_chars argument")?;

        Ok(Expression::StringLenChars(StringLenCharsExpr {
            string,
            location,
        }))
    }

    /// Parse string concatenation expression: concat(a, b)
    fn parse_string_concat(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let a = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after first argument in concat")?;
        let b = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after concat arguments")?;

        Ok(Expression::StringConcat(StringConcatExpr {
            a,
            b,
            location,
        }))
    }

    /// Parse string substring expression: substring(s, start, end)
    fn parse_string_substring(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after string argument in substring")?;
        let start = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after start index in substring")?;
        let end = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after substring arguments")?;

        Ok(Expression::StringSubstring(StringSubstringExpr {
            string,
            start,
            end,
            location,
        }))
    }

    /// Parse string substring until character expression: substring_until_char(s, start, char)
    fn parse_string_substring_until_char(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after string argument in substring_until_char")?;
        let start = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after start index in substring_until_char")?;
        let char_expr = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after substring_until_char arguments")?;

        Ok(Expression::StringSubstringUntilChar(StringSubstringUntilCharExpr {
            string,
            start,
            char: char_expr,
            location,
        }))
    }

    /// Parse string starts with expression: starts_with(s, prefix)
    fn parse_string_starts_with(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after string argument in starts_with")?;
        let prefix = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after starts_with arguments")?;

        Ok(Expression::StringStartsWith(StringStartsWithExpr {
            string,
            prefix,
            location,
        }))
    }

    /// Parse string ends with expression: ends_with(s, suffix)
    fn parse_string_ends_with(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after string argument in ends_with")?;
        let suffix = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after ends_with arguments")?;

        Ok(Expression::StringEndsWith(StringEndsWithExpr {
            string,
            suffix,
            location,
        }))
    }

    /// Parse string contains expression: contains(s, substr)
    fn parse_string_contains(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let string = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after string argument in contains")?;
        let substr = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after contains arguments")?;

        Ok(Expression::StringContains(StringContainsExpr {
            string,
            substr,
            location,
        }))
    }

    /// Parse exec_command(command) expression - simplified for now
    fn parse_exec_command(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let command = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after exec_command argument")?;

        Ok(Expression::ExecCommand(ExecCommandExpr {
            command,
            args: Vec::new(), // No args for now
            location,
        }))
    }

    /// Parse constructor call: TypeName::Constructor<Args>(payload)
    fn parse_constructor_call(&mut self, type_name: String, location: SourceLocation) -> Result<Expression> {
        // Parse constructor name
        let constructor = self.consume_identifier("Expected constructor name after '::'")?;

        // Parse optional type arguments
        let type_args = if self.match_token(TokenKind::Less) {
            let mut args = Vec::new();
            if !self.check(TokenKind::Greater) {
                loop {
                    args.push(self.parse_type()?);
                    if !self.match_token(TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.consume(TokenKind::Greater, "Expected '>' after type arguments")?;
            args
        } else {
            Vec::new()
        };

        // Parse payload (required for constructors)
        self.consume(TokenKind::LeftParen, "Expected '(' after constructor name")?;
        let payload = if self.check(TokenKind::RightParen) {
            None
        } else {
            let expr = self.expression()?;
            Some(Box::new(expr))
        };
        self.consume(TokenKind::RightParen, "Expected ')' after constructor arguments")?;

        Ok(Expression::ConstructorCall(ConstructorCallExpr {
            type_name,
            constructor,
            type_args,
            payload,
            location,
        }))
    }


    /// Parse core_id(core_number) expression
    fn parse_core_id(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let core_expr = self.expression()?;
        self.consume(TokenKind::RightParen, "Expected ')' after core_id argument")?;

        // For now, we'll represent this as a call expression
        // In the future, this could be a dedicated AST node
        Ok(Expression::Call(CallExpr {
            function: Box::new(Expression::Identifier("core_id".to_string())),
            arguments: vec![core_expr],
            location,
        }))
    }

    /// Parse spawn(initial_state, behavior[, core_affinity]) expression
    fn parse_spawn(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::LeftParen, "Expected '(' after 'spawn'")?;
        let initial_state = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after initial_state in spawn")?;
        let behavior = Box::new(self.expression()?);

        // Check for optional third parameter (core affinity)
        let core_affinity = if self.match_token(TokenKind::Comma) {
            Some(Box::new(self.expression()?))
        } else {
            None
        };

        self.consume(TokenKind::RightParen, "Expected ')' after spawn arguments")?;

        Ok(Expression::Spawn(SpawnExpr {
            initial_state,
            behavior,
            core_affinity,
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

    /// Parse recv() or recv(actor) expression
    fn parse_recv(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::LeftParen, "Expected '(' after 'recv'")?;

        let actor = if self.check(TokenKind::RightParen) {
            // recv() - no actor specified
            None
        } else {
            // recv(actor) - parse actor expression
            Some(Box::new(self.expression()?))
        };

        self.consume(TokenKind::RightParen, "Expected ')' after recv")?;

        Ok(Expression::Recv(RecvExpr {
            actor,
            location,
        }))
    }

    /// Parse cast(actor, message) expression
    fn parse_cast(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        self.consume(TokenKind::LeftParen, "Expected '(' after 'cast'")?;
        let actor = Box::new(self.expression()?);
        self.consume(TokenKind::Comma, "Expected ',' after actor in cast")?;
        let message = Box::new(self.expression()?);
        self.consume(TokenKind::RightParen, "Expected ')' after cast arguments")?;

        Ok(Expression::Cast(CastExpr {
            actor,
            message,
            location,
        }))
    }

    /// Parse struct declaration: struct Name<T, U> { field: Type, ... }
    fn struct_declaration(&mut self) -> Result<StructDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected struct name")?;


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
            fields,
            location,
        })
    }

    /// Parse enum declaration: enum Name<T, U> { Variant1, Variant2(Type), Variant3 { field: Type } }
    fn enum_declaration(&mut self) -> Result<EnumDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected enum name")?;


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
            variants,
            location,
        })
    }

    /// Parse trait declaration: trait Name includes Trait1, Trait2 { fn method(&self, param: Type) -> ReturnType; }
    fn trait_declaration(&mut self) -> Result<TraitDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected trait name")?;

        // Parse optional included traits
        let included_traits = if self.match_token(TokenKind::Includes) {
            let mut traits = Vec::new();
            loop {
                let trait_name = self.consume_identifier("Expected trait name after 'includes'")?;
                traits.push(trait_name);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
            traits
        } else {
            Vec::new()
        };

        self.consume(TokenKind::LeftBrace, "Expected '{' after trait name")?;

        let mut associated_types = Vec::new();
        let mut methods = Vec::new();
        if !self.check(TokenKind::RightBrace) {
            loop {
                if self.match_token(TokenKind::Type) {
                    // Parse associated type
                    let type_name = self.consume_identifier("Expected associated type name")?;
                    let type_location = self.previous().location.clone();

                    // Parse optional bounds (: Trait1 + Trait2)
                    let mut bounds = Vec::new();
                    if self.match_token(TokenKind::Colon) {
                        loop {
                            let trait_name = self.consume_identifier("Expected trait name in bound")?;
                            bounds.push(trait_name);
                            if !self.match_token(TokenKind::Plus) {
                                break;
                            }
                        }
                    }

                    self.consume(TokenKind::Semicolon, "Expected ';' after associated type")?;

                    associated_types.push(AssociatedType {
                        name: type_name,
                        bounds,
                        location: type_location,
                    });
                } else if self.match_token(TokenKind::Fn) {
                    // Parse trait method
                let method_name = self.consume_identifier("Expected method name")?;
                let method_location = self.previous().location.clone();

                self.consume(TokenKind::LeftParen, "Expected '(' after method name")?;

                let mut params = Vec::new();
                if !self.check(TokenKind::RightParen) {
                    loop {
                        // Allow 'self' as a special parameter name
                        let param_name = if self.match_token(TokenKind::Self_) {
                            "self".to_string()
                        } else {
                            self.consume_identifier("Expected parameter name")?
                        };

                        // Require explicit type annotation for ALL parameters, including self
                        self.consume(TokenKind::Colon, "Expected ':' after parameter name")?;
                        let param_type = self.parse_type()?;

                        params.push(Parameter {
                            name: param_name,
                            type_: param_type,
                            location: self.previous().location.clone(),
                            pattern: None,
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
                } else {
                    let location = self.peek().location.clone();
                    let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§7"), None)
                        .build();
                    return parse_error_with_metadata(location, "Expected 'type' or 'fn' in trait declaration".to_string(), metadata);
                }

                if self.check(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after trait members")?;

        Ok(TraitDecl {
            name,
            included_traits,
            associated_types,
            methods,
            location,
        })
    }

    /// Parse impl declaration: impl<T> Trait for Type { ... } or impl Type { ... }
    fn impl_declaration(&mut self) -> Result<ImplDecl> {
        // eprintln!("DEBUG IMPL: impl_declaration called");
        let location = self.previous().location.clone();


        // Check if this is a trait implementation
        let trait_name = if let TokenKind::Identifier(name) = &self.peek().kind {
            let name_clone = name.clone();
            self.advance();
            if self.match_token(TokenKind::For) {
                Some(name_clone)
            } else {
                // This was actually the type name, not a trait name
                let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§7"), None)
                    .build();
                return parse_error_with_metadata(location, "Expected 'for' after trait name in impl".to_string(), metadata);
            }
        } else {
            None // Inherent impl
        };

        // Parse the type being implemented for (simplified)
        let token = self.peek().clone();
        self.advance();
        let for_type = match token.kind {
            TokenKind::Identifier(name) => Type::Named(name),
            _ => {
                let metadata = self.build_parse_error_metadata("E1003", &token.location, Some("spec:§3"), None)
                    .build();
                return parse_error_with_metadata(token.location, "Expected type name".to_string(), metadata);
            }
        };
        
        let mut associated_types: Vec<crate::ast::AssociatedTypeDef> = Vec::new();
        let mut methods = Vec::new();

        // Require braces for impl blocks (even if empty for marker traits)
        self.consume(TokenKind::LeftBrace, "Expected '{' after impl type")?;

        // Parse methods properly in impl blocks
        // eprintln!("DEBUG IMPL: Starting method parsing loop");
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            let current_token = self.peek();
            // eprintln!("DEBUG IMPL: Current token: {:?} (kind={:?}) at line {} col {}",
            //          current_token.lexeme, current_token.kind, current_token.location.line, current_token.location.column);
            if self.match_token(TokenKind::Fn) {
                // eprintln!("DEBUG IMPL: Successfully matched Fn token");
                // Use the standard function declaration parser for methods
                let method = self.function_declaration()?;
                // eprintln!("DEBUG IMPL: Successfully parsed method: {}", method.name);
                methods.push(method);
            } else {
                // eprintln!("DEBUG IMPL: Failed to match Fn token, advancing");
                // Skip unrecognized tokens
                self.advance();
            }
        }
        // eprintln!("DEBUG IMPL: Finished method parsing loop, found {} methods", methods.len());

        self.consume(TokenKind::RightBrace, "Expected '}' after impl members")?;


        Ok(ImplDecl {
            trait_name,
            for_type,
            associated_types,
            methods,
            location,
        })
    }

    /// Parse type alias declaration: type Name<T, U> = Type;
    fn type_alias_declaration(&mut self) -> Result<TypeAliasDecl> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected type alias name")?;


        self.consume(TokenKind::Equal, "Expected '=' after type alias name")?;
        let aliased_type = self.parse_type()?;
        self.consume(TokenKind::Semicolon, "Expected ';' after type alias")?;

        Ok(TypeAliasDecl {
            name,
            aliased_type,
            location,
        })
    }

    /// Parse type parameters: <T, U, V>



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

    /// Parse case expression with pattern matching
    fn case_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let scrutinee = Box::new(self.expression()?);
        self.consume(TokenKind::Of, "Expected 'of' after case scrutinee")?;
        self.consume(TokenKind::LeftBrace, "Expected '{' after 'of'")?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            let pattern = self.pattern()?;

            // Check for optional guard
            let guard = if self.match_token(TokenKind::If) {
                Some(Box::new(self.expression()?))
            } else {
                None
            };

            self.consume(TokenKind::RightArrow, "Expected '->' after pattern")?;
            let body = self.expression()?;

            branches.push(CaseBranch {
                pattern,
                guard,
                body: Box::new(body),
                location: self.previous().location.clone(),
            });

            // Semicolon is optional between branches
            self.match_token(TokenKind::Semicolon);
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after case branches")?;

        Ok(Expression::Case(CaseExpr {
            scrutinee,
            branches,
            location,
        }))
    }

    /// Parse do expression
    fn parse_statements(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();

        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            // Validate current token position
            let current_token = self.peek();
            let error_location = current_token.location.clone(); // Clone location before any mutable operations
            if current_token.location.line > 10000 {  // Sanity check
                let metadata = ErrorMetadataBuilder::new("E9002".to_string())
                    .severity(ErrorSeverity::Error)
                    .build();
                return parse_error_with_metadata(error_location,
                    "Parser position tracking corrupted during statement parsing".to_string(),
                    metadata);
            }
            
            // Reject nested function declarations (fn keyword followed by identifier)
            // Function declarations are only allowed at top-level, not inside function bodies
            if self.check(TokenKind::Fn) {
                let saved_pos = self.current;
                self.advance(); // consume 'fn'
                if matches!(self.peek().kind, TokenKind::Identifier(_)) {
                    let metadata = ErrorMetadataBuilder::new("E1002".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§3.4.1".to_string(), Some("Function Declarations".to_string()))
                        .suggestion("Move the function to top-level or use a function literal (lambda) instead".to_string())
                        .suggestion_with_example("Use function literal:".to_string(), "fn helper() { fn(x: int) { x + 1 } }".to_string())
                        .build();
                    return parse_error_with_metadata(
                        error_location,
                        "Nested function declarations are not allowed in Silica. Functions must be declared at top-level.".to_string(),
                        metadata,
                    );
                }
                // Not a function declaration, backtrack and parse as function literal
                self.current = saved_pos;
            }
            
            // Try to parse assignment first
            let current_pos = self.current;
            // Check if this could be a pattern (identifier, tuple, or wildcard)
            // Only attempt pattern parsing if identifier is followed by colon (type annotation)
            // or if it's a tuple/wildcard pattern, since bindings require type annotations
            let could_be_binding = match self.peek().kind {
                TokenKind::Identifier(_) => {
                    // For identifiers, check if colon follows (indicating type annotation)
                    if self.current + 1 < self.tokens.len() {
                        self.tokens[self.current + 1].kind == TokenKind::Colon
                    } else {
                        false
                    }
                },
                TokenKind::LeftParen | TokenKind::Underscore => true, // Tuples and wildcards can be patterns
                _ => false,
            };
            
            if could_be_binding {
                // Try to parse a pattern followed by '<-'
                let saved_pos = self.current;
                if let Ok(pattern) = self.pattern() {
                    if self.match_token(TokenKind::LeftArrow) {
                        let expr = self.expression()?;
                        self.match_token(TokenKind::Semicolon); // Semicolon is optional
                        statements.push(Statement::Bind {
                            pattern,
                            expr: Box::new(expr),
                        });
                        continue;
                    } else {
                        // Not a binding, backtrack
                        self.current = saved_pos;
                    }
                } else {
                    // Not a valid pattern, backtrack
                    self.current = saved_pos;
                }
            }

            // Parse as regular expression statement
            let current_token = self.peek();
            eprintln!("DEBUG PARSER: parse_statements() parsing expression statement at {}:{} (token: {:?} '{}')", 
                     current_token.location.line, current_token.location.column, 
                     current_token.kind, current_token.lexeme);
            let expr = self.expression()?;
            eprintln!("DEBUG PARSER: parse_statements() expression parsed successfully");
            // Semicolon is required for all statements except the last one
            if !self.check(TokenKind::RightBrace) {
                // Try to consume semicolon, but provide better error recovery
                if let Err(_) = self.consume(TokenKind::Semicolon, "Expected ';' after statement") {
                    // If semicolon consumption fails, check if we're at end of input or another valid token
                    let current = self.peek();
                    if current.kind == TokenKind::EOF || current.kind == TokenKind::RightBrace {
                        // We're at a valid stopping point, continue without semicolon
                        // This provides better error recovery for position tracking issues
                    } else {
                        // Check for common syntax errors and provide helpful messages
                        let (error_msg, error_code, suggestion) = match current.kind {
                            TokenKind::Identifier(ref id) if id == "if" => {
                                ("Found 'if' keyword, but Silica does not support if-else statements. Use 'case' expressions instead: case condition of true -> ... false -> ...".to_string(),
                                 "E1008".to_string(),
                                 "Use 'case' expressions: case condition of { true -> ... false -> ... }".to_string())
                            },
                            TokenKind::Identifier(ref id) if id == "else" => {
                                ("Found 'else' keyword, but Silica does not support if-else statements. Use 'case' expressions instead.".to_string(),
                                 "E1008".to_string(),
                                 "Use 'case' expressions instead of if-else".to_string())
                            },
                            TokenKind::Identifier(ref id) if id == "for" => {
                                ("Found 'for' keyword, but Silica does not support for loops. Use recursion instead.".to_string(),
                                 "E1008".to_string(),
                                 "Use recursion instead of for loops".to_string())
                            },
                            TokenKind::Identifier(ref id) if id == "while" => {
                                ("Found 'while' keyword, but Silica does not support while loops. Use recursion instead.".to_string(),
                                 "E1008".to_string(),
                                 "Use recursion instead of while loops".to_string())
                            },
                            _ => (format!("Expected ';' after statement, found {:?}", current.kind),
                                  "E1001".to_string(),
                                  "Add semicolon (;) after the statement".to_string())
                        };
                        let metadata = ErrorMetadataBuilder::new(error_code)
                            .severity(ErrorSeverity::Error)
                            .specification("§3".to_string(), None)
                            .suggestion(suggestion)
                            .build();
                        return parse_error_with_metadata(self.validate_token_position(current), error_msg, metadata);
                    }
                }
            }
            statements.push(Statement::Expr(Box::new(expr)));
        }

        Ok(statements)
    }

    fn do_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let mut statements = Vec::new();

        while !self.check(TokenKind::End) && !self.is_at_end() {
            // Try to parse assignment first
            let current_pos = self.current;
            // Check if this could be a pattern (identifier, tuple, or wildcard)
            if matches!(self.peek().kind, TokenKind::Identifier(_) | TokenKind::LeftParen | TokenKind::Underscore) {
                // Try to parse a pattern followed by '<-'
                let saved_pos = self.current;
                if let Ok(pattern) = self.pattern() {
                    if self.match_token(TokenKind::LeftArrow) {
                        let expr = self.expression()?;
                        if !self.check(TokenKind::End) {
                            self.consume(TokenKind::Semicolon, "Expected ';' after binding")?;
                        }
                        statements.push(Statement::Bind {
                            pattern,
                            expr: Box::new(expr),
                        });
                        continue;
                    } else {
                        // Not a binding, backtrack
                        self.current = saved_pos;
                    }
                } else {
                    // Not a valid pattern, backtrack
                    self.current = saved_pos;
                }
            }

            // Parse as regular expression statement
                let expr = self.expression()?;
            // Semicolon is optional before 'end'
            self.match_token(TokenKind::Semicolon);
                statements.push(Statement::Expr(Box::new(expr)));
        }

        self.consume(TokenKind::End, "Expected 'end' after do block")?;

        Ok(Expression::Do(DoExpr {
            statements,
            location,
        }))
    }

    /// Parse function literal (lambda/anonymous function)
    fn function_literal(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();


        self.consume(TokenKind::LeftParen, "Expected '(' after 'fn'")?;
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


        // Parse function body
        self.consume(TokenKind::LeftBrace, "Expected '{' after function signature")?;
        let body = self.parse_statements()?;
        self.consume(TokenKind::RightBrace, "Expected '}' after function body")?;

        // Captured variables will be detected during type checking/code generation
        let captured_vars = Vec::new();

        Ok(Expression::FunctionLiteral(FunctionLiteralExpr {
            parameters,
            return_type,
            body,
            effects,
            captured_vars,
            location,
        }))
    }

    /// Parse pattern
    fn pattern(&mut self) -> Result<Pattern> {
        // Handle literal patterns
        match &self.peek().kind {
            TokenKind::IntegerLiteral(value) => {
                let value = *value;
                self.advance();
                return Ok(Pattern::Literal(Literal::Int(value)));
            }
            TokenKind::FloatLiteral(value) => {
                let value = *value;
                self.advance();
                return Ok(Pattern::Literal(Literal::Float(value)));
            }
            TokenKind::True => {
                self.advance();
                return Ok(Pattern::Literal(Literal::Bool(true)));
            }
            TokenKind::False => {
                self.advance();
                return Ok(Pattern::Literal(Literal::Bool(false)));
            }
            TokenKind::StringLiteral(value) => {
                let value = value.clone();
                self.advance();
                return Ok(Pattern::Literal(Literal::String(value)));
            }
            TokenKind::CharLiteral(value) => {
                let value = *value;
                self.advance();
                return Ok(Pattern::Literal(Literal::Char(value)));
            }
            _ => {} // Fall through to handle other pattern types (identifiers, wildcards, tuples, etc.)
        }

        if matches!(&self.peek().kind, TokenKind::Identifier(_) | TokenKind::Underscore) {
            let (name, start_location) = match &self.peek().kind {
                TokenKind::Identifier(n) => (n.clone(), self.peek().location.clone()),
                TokenKind::Underscore => ("_".to_string(), self.peek().location.clone()),
                _ => unreachable!(),
            };
            self.advance();

            // Type annotation is optional for simple identifiers, required for wildcards
            if self.match_token(TokenKind::Colon) {
                let type_ = self.parse_type()?;

                // Check for regular variant: Constructor(payload)
                if self.match_token(TokenKind::LeftParen) {
                    let payload = if self.check(TokenKind::RightParen) {
                        None
                    } else {
                        let pat = self.pattern()?;
                        Some(Box::new(pat))
                    };
                    self.consume(TokenKind::RightParen, "Expected ')' after variant payload")?;

                    return Ok(Pattern::Variant {
                        constructor: name,
                        payload,
                    });
                } else {
                    // Typed identifier
                    return Ok(Pattern::TypedIdentifier { name, type_ });
                }
            } else {
                // No type annotation
                if name == "_" {
                    let metadata = ErrorMetadataBuilder::new("E1007".to_string())
                        .severity(ErrorSeverity::Error)
                        .specification("§3".to_string(), None)  // Display will add "spec:" prefix
                        .suggestion("Add explicit type annotation: _: Type".to_string())
                        .suggestion_with_example("Example:".to_string(), "let _: int <- get_value();".to_string())
                        .build();
                    return parse_error_with_metadata(start_location, "Wildcards must have explicit type annotations: _: Type".to_string(), metadata);
                }
                // Untyped identifier
                return Ok(Pattern::Identifier(name));
            }
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
            let location = self.peek().location.clone();
            let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§3"), None)
                .build();
            parse_error_with_metadata(
                location,
                "Expected pattern".to_string(),
                metadata,
            )
        }
    }

    /// Collect variables captured by a function literal
    fn collect_captured_vars(&self, body: &Expression, parameters: &[Parameter]) -> Vec<String> {
        let mut used_vars = std::collections::HashSet::new();
        let mut defined_vars = std::collections::HashSet::new();

        // Collect parameter names
        for param in parameters {
            defined_vars.insert(param.name.clone());
        }

        // Collect all identifiers used in the body
        self.collect_identifiers(body, &mut used_vars);

        // Return variables that are used but not defined locally
        used_vars.into_iter()
            .filter(|var| !defined_vars.contains(var))
            .collect()
    }

    fn collect_captured_vars_from_statements(&self, statements: &[Statement], parameters: &[Parameter]) -> Vec<String> {
        let mut used_vars = std::collections::HashSet::new();
        let mut defined_vars = std::collections::HashSet::new();

        // Collect parameter names
        for param in parameters {
            defined_vars.insert(param.name.clone());
        }

        // Collect all identifiers used in the statements
        for statement in statements {
            self.collect_identifiers_from_statement(statement, &mut used_vars);
            // Add bound variables to defined vars
            if let Statement::Bind { pattern, .. } = statement {
                self.collect_bound_vars_from_pattern(pattern, &mut defined_vars);
            }
        }

        // Return variables that are used but not defined locally
        used_vars.into_iter()
            .filter(|var| !defined_vars.contains(var))
            .collect()
    }

    /// Recursively collect all identifiers used in a statement
    fn collect_identifiers_from_statement(&self, statement: &Statement, identifiers: &mut std::collections::HashSet<String>) {
        match statement {
            Statement::Bind { expr, .. } => {
                self.collect_identifiers(expr, identifiers);
            }
            Statement::Expr(expr) => {
                self.collect_identifiers(expr, identifiers);
            }
        }
    }

    /// Collect bound variables from a pattern
    fn collect_bound_vars_from_pattern(&self, pattern: &Pattern, bound_vars: &mut std::collections::HashSet<String>) {
        match pattern {
            Pattern::Identifier(name) => {
                bound_vars.insert(name.clone());
            }
            Pattern::TypedIdentifier { name, .. } => {
                if name != "_" {
                    bound_vars.insert(name.clone());
                }
            }
            Pattern::Tuple(patterns) => {
                for pattern in patterns {
                    self.collect_bound_vars_from_pattern(pattern, bound_vars);
                }
            }
            Pattern::Literal(_) => {
                // Literals don't bind variables
            }
            Pattern::Record(fields) => {
                for (_, field_pattern) in fields {
                    self.collect_bound_vars_from_pattern(field_pattern, bound_vars);
                }
            }
            Pattern::Variant { payload, .. } => {
                if let Some(payload_pattern) = payload {
                    self.collect_bound_vars_from_pattern(payload_pattern, bound_vars);
                }
            }
            Pattern::Alternative(patterns) => {
                for pattern in patterns {
                    self.collect_bound_vars_from_pattern(pattern, bound_vars);
                }
            }
        }
    }

    /// Recursively collect all identifiers used in an expression
    fn collect_identifiers(&self, expr: &Expression, identifiers: &mut std::collections::HashSet<String>) {
        match expr {
            Expression::Identifier(name) => {
                identifiers.insert(name.clone());
            }
            Expression::Call(call) => {
                self.collect_identifiers(&call.function, identifiers);
                for arg in &call.arguments {
                    self.collect_identifiers(arg, identifiers);
                }
            }
            Expression::Binary(binary) => {
                self.collect_identifiers(&binary.left, identifiers);
                self.collect_identifiers(&binary.right, identifiers);
            }
            Expression::Unary(unary) => {
                self.collect_identifiers(&unary.operand, identifiers);
            }
            Expression::If(if_expr) => {
                self.collect_identifiers(&if_expr.condition, identifiers);
                self.collect_identifiers(&if_expr.then_branch, identifiers);
                self.collect_identifiers(&if_expr.else_branch, identifiers);
            }
            Expression::Case(case) => {
                self.collect_identifiers(&case.scrutinee, identifiers);
                for branch in &case.branches {
                    self.collect_identifiers(&branch.body, identifiers);
                }
            }
            Expression::Do(do_expr) => {
                for stmt in &do_expr.statements {
                    match stmt {
                        Statement::Expr(expr) => self.collect_identifiers(expr, identifiers),
                        Statement::Bind { pattern: _, expr } => {
                            self.collect_identifiers(expr, identifiers);
                            // Note: We don't add bound variables to defined_vars here since
                            // this is a simple analysis and bindings create new scopes
                        }
                    }
                }
            }
            Expression::FunctionLiteral(func) => {
                // Don't recurse into nested function literals for capture analysis
                // (they would have their own capture analysis)
            }
            // Other expressions don't contain identifiers we care about
            _ => {}
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

    /// Peek with validated position
    fn peek_valid(&self) -> &Token {
        let token = self.peek();
        // Note: We can't modify the token in place, but we can validate it when used
        token
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<&Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let location = self.peek().location.clone();
            let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§3"), Some(message))
                .build();
            parse_error_with_metadata(location, message.to_string(), metadata)
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String> {
        // Special case for &self - return a special marker
        if self.check(TokenKind::Ampersand) && self.tokens.get(self.current + 1).map_or(false, |t| t.kind == TokenKind::Self_) {
            self.advance(); // consume &
            self.advance(); // consume self
            return Ok("&self".to_string()); // Special marker for reference self
        }

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
            let location = self.peek().location.clone();
            let metadata = self.build_parse_error_metadata("E1001", &location, Some("spec:§3"), Some(message))
                .build();
            parse_error_with_metadata(location, message.to_string(), metadata)
        }
    }

    /// Create error metadata builder for parse errors
    fn build_parse_error_metadata(&self, error_code: &str, location: &SourceLocation, spec_section: Option<&str>, suggestion: Option<&str>) -> ErrorMetadataBuilder {
        let mut builder = ErrorMetadataBuilder::new(error_code.to_string())
            .severity(ErrorSeverity::Error);
        
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
        
        // Add suggestion if provided
        if let Some(sug) = suggestion {
            builder = builder.suggestion(sug.to_string());
        }
        
        builder
    }
}
