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
            self.impl_declaration().map(Declaration::Impl)
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

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

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

        // Parse optional where clause
        let where_clause = if self.match_token(TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        self.consume(TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.expression()?;
        self.consume(TokenKind::RightBrace, "Expected '}' after function body")?;

        Ok(FunctionDecl {
            name,
            type_params,
            parameters,
            return_type,
            where_clause,
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

        // Check if this is a &self parameter (special marker from consume_identifier)
        if name == "&self" {
            return Ok(Parameter {
                name: "self".to_string(),
                type_: Type::Named("&Self".to_string()), // Placeholder for reference to Self
                location,
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
        // Reference type: &Type
        if self.match_token(TokenKind::Ampersand) {
            let element_type = self.parse_type()?;
            // For now, create a placeholder reference type
            // In a full implementation, this would create a proper Reference type
            let type_str = match &element_type {
                Type::Named(name) => name.clone(),
                Type::Unit => "unit".to_string(),
                Type::Bool => "bool".to_string(),
                Type::Int => "int".to_string(),
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
    fn parse_type_with_params(&mut self, type_params: &[String]) -> Result<Type> {
        if self.match_token(TokenKind::LeftParen) {
            // Tuple type or function type
            if self.match_token(TokenKind::RightParen) {
                // Unit type
                Ok(Type::Unit)
            } else {
                let first_type = self.parse_type_with_params(type_params)?;
                if self.match_token(TokenKind::Comma) {
                    // Tuple type
                    let mut types = vec![first_type];
                    loop {
                        types.push(self.parse_type_with_params(type_params)?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                    self.consume(TokenKind::RightParen, "Expected ')' after tuple types")?;
                    Ok(Type::Tuple(types))
                } else {
                    // Function type
                    self.consume(TokenKind::RightArrow, "Expected '->' in function type")?;
                    let return_type = self.parse_type_with_params(type_params)?;
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
            let result_type = self.parse_type_with_params(type_params)?;
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
                let field_type = self.parse_type_with_params(type_params)?;
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
            let region = self.parse_type_with_params(type_params)?;
            self.consume(TokenKind::Comma, "Expected ',' after region")?;
            let space = self.memory_space()?;
            self.consume(TokenKind::Comma, "Expected ',' after memory space")?;
            let element_type = self.parse_type_with_params(type_params)?;
            self.consume(TokenKind::RightParen, "Expected ')' after reference type")?;
            Ok(Type::Reference {
                region: Box::new(region),
                space,
                element_type: Box::new(element_type),
            })
        } else if self.match_token(TokenKind::Buf) {
            // Buffer type
            self.consume(TokenKind::LeftParen, "Expected '(' after 'buf'")?;
            let region = self.parse_type_with_params(type_params)?;
            self.consume(TokenKind::Comma, "Expected ',' after region")?;
            let space = self.memory_space()?;
            self.consume(TokenKind::Comma, "Expected ',' after memory space")?;
            let element_type = self.parse_type_with_params(type_params)?;
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
            let message_type = self.parse_type_with_params(type_params)?;
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
            // User-defined type name or type variable
            let name = self.consume_identifier("Expected type name")?;

            // Check if this is a type parameter
            if type_params.contains(&name) {
                Ok(Type::Variable(name))
            } else {
            Ok(Type::Named(name))
            }
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
                Expression::Identifier(name) => {
                    let mut statements = vec![
                        Statement::Bind {
                            pattern: Pattern::Identifier(name),
                            expr: Box::new(value),
                        }
                    ];

                    // Check for semicolon and additional expressions
                    if self.match_token(TokenKind::Semicolon) {
                        let next_expr = self.assignment()?;
                        statements.push(Statement::Expr(Box::new(next_expr)));
                    }

                    Ok(Expression::Do(DoExpr {
                        statements,
                    location: self.previous().location.clone(),
                    }))
                }
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

    /// Parse function calls, field access, and primary expressions
    fn call(&mut self) -> Result<Expression> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(TokenKind::Less) {
                expr = self.finish_generic_call(expr)?;
            } else if self.match_token(TokenKind::Dot) {
                expr = self.finish_field_access(expr)?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    /// Finish parsing a function call
    fn finish_call(&mut self, callee: Expression) -> Result<Expression> {
        let mut arguments = Vec::new();
        let mut type_args = Vec::new();
        let location = self.previous().location.clone();

        // Check for optional type arguments
        if self.match_token(TokenKind::Less) {
            type_args = self.parse_type_arguments()?;
        }

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
            type_args,
            arguments,
            location,
        }))
    }

    /// Finish parsing generic function call: func<type_args>(args)
    fn finish_generic_call(&mut self, function_expr: Expression) -> Result<Expression> {
        let location = self.previous().location.clone();

        // Parse type arguments
        let type_args = self.parse_type_arguments()?;
        self.consume(TokenKind::LeftParen, "Expected '(' after type arguments")?;

        // Parse regular arguments
        let mut arguments = Vec::new();
        if !self.check(TokenKind::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::RightParen, "Expected ')' after arguments")?;

        // For now, only support identifier functions
        match function_expr {
            Expression::Identifier(function_name) => {
                Ok(Expression::Call(CallExpr {
                    function: Box::new(Expression::Identifier(function_name)),
                    type_args,
                    arguments,
                    location,
                }))
            }
            _ => {
                parse_error(location, "Generic calls only supported on identifiers".to_string())
            }
        }
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
            _ => return parse_error(
                location.clone(),
                "Expected type name before struct literal".to_string(),
            ),
        };

        let mut fields = Vec::new();

    // Temporarily disable field parsing
    // if !self.check(TokenKind::RightBrace) {
    //     loop {
    //         let field_name = self.consume_identifier("Expected field name")?;
    //         self.consume(TokenKind::Colon, "Expected ':' after field name")?;
    //         let field_value = self.expression()?;
    //
    //         fields.push((field_name, field_value));
    //
    //         if self.check(TokenKind::RightBrace) {
    //             break;
    //         }
    //         self.consume(TokenKind::Comma, "Expected ',' after field")?;
    //     }
    // }

        self.consume(TokenKind::RightBrace, "Expected '}' after struct literal")?;

        Ok(Expression::StructLiteral(StructLiteralExpr {
            type_name,
            fields,
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
        } else if self.match_token(TokenKind::If) {
            self.if_expression()
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
            // Generate a built-in region allocation operation
            // For now, return a placeholder pointer - in full implementation this would
            // allocate a region handle directly
            Ok(Expression::Literal(Literal::Int(0)))
        } else if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            let start_location = self.peek().location.clone();
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
            } else if name == "write_ref" && self.match_token(TokenKind::LeftParen) {
                self.parse_write_ref()
            } else if self.match_token(TokenKind::LeftBrace) {
                // Parse struct literal - simplified for now
                self.consume(TokenKind::RightBrace, "Expected '}' after struct literal")?;
                Ok(Expression::StructLiteral(StructLiteralExpr {
                    type_name: name,
                    fields: Vec::new(),
                    location: start_location,
                }))
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
        } else if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
                Ok(Expression::Identifier(name))
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

    /// Parse generic instantiation: TypeName<Arg1, Arg2>(payload)
    fn parse_generic_instantiation(&mut self, type_name: String, location: SourceLocation) -> Result<Expression> {
        // Parse type arguments
        let mut type_args = Vec::new();
        if !self.check(TokenKind::Greater) {
            loop {
                type_args.push(self.parse_type()?);
                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenKind::Greater, "Expected '>' after type arguments")?;

        // Parse optional payload (for enum variants like Some(42))
        let payload = if self.match_token(TokenKind::LeftParen) {
            let expr = self.expression()?;
            self.consume(TokenKind::RightParen, "Expected ')' after payload")?;
            Some(Box::new(expr))
        } else {
            None
        };

        Ok(Expression::GenericInstantiation(GenericInstantiationExpr {
            type_name,
            type_args,
            payload,
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
            let field_type = self.parse_type_with_params(&type_params)?;

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
                            fields.push(self.parse_type_with_params(&type_params)?);
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
                } else {
                    return parse_error(self.peek().location.clone(), "Expected 'type' or 'fn' in trait declaration".to_string());
                }

                if self.check(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after trait members")?;

        Ok(TraitDecl {
            name,
            type_params,
            associated_types,
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

        let mut associated_types = Vec::new();
        let mut methods = Vec::new();
        if !self.check(TokenKind::RightBrace) {
            loop {
                if self.match_token(TokenKind::Type) {
                    // Parse associated type definition
                    let type_name = self.consume_identifier("Expected associated type name")?;
                    let type_location = self.previous().location.clone();

                    self.consume(TokenKind::Equal, "Expected '=' in associated type definition")?;
                    let type_value = self.parse_type()?;
                    self.consume(TokenKind::Semicolon, "Expected ';' after associated type definition")?;

                    associated_types.push(AssociatedTypeDef {
                        name: type_name,
                        type_: type_value,
                        location: type_location,
                    });
                } else {
                    // Parse method
                let method = self.function_declaration()?;
                methods.push(method);
                }

                if self.check(TokenKind::RightBrace) {
                    break;
                }
            }
        }

        self.consume(TokenKind::RightBrace, "Expected '}' after impl members")?;

        Ok(ImplDecl {
            trait_name,
            type_params,
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

    /// Parse type arguments: <int, string, T>
    fn parse_type_arguments(&mut self) -> Result<Vec<Type>> {
        let mut args = Vec::new();

        if !self.check(TokenKind::Greater) {
            loop {
                let ty = self.parse_type()?;
                args.push(ty);

                if !self.match_token(TokenKind::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenKind::Greater, "Expected '>' after type arguments")?;
        Ok(args)
    }

    /// Parse where clause
    fn parse_where_clause(&mut self) -> Result<WhereClause> {
        let mut predicates = Vec::new();

        loop {
            // Parse type : bounds
            let type_ = self.parse_type()?;
            self.consume(TokenKind::Colon, "Expected ':' after type in where clause")?;

            let mut bounds = Vec::new();
            loop {
                let trait_name = self.consume_identifier("Expected trait name in bound")?;
                let type_args = if self.match_token(TokenKind::Less) {
                    self.parse_type_arguments()?
                } else {
                    Vec::new()
                };

                bounds.push(TraitBound {
                    trait_name,
                    type_args,
                });

                if !self.match_token(TokenKind::Plus) {
                    break;
                }
            }

            predicates.push(WherePredicate::TraitBound { type_, bounds });

            if !self.match_token(TokenKind::Comma) {
                break;
            }
        }

        Ok(WhereClause { predicates })
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

    /// Parse case expression with pattern matching
    fn case_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let scrutinee = Box::new(self.expression()?);
        self.consume(TokenKind::LeftBrace, "Expected '{' after case scrutinee")?;

        let mut branches = Vec::new();
        while !self.check(TokenKind::RightBrace) && !self.is_at_end() {
            let pattern = self.pattern()?;

            // Check for optional guard
            let guard = if self.match_token(TokenKind::If) {
                Some(Box::new(self.expression()?))
            } else {
                None
            };

            self.consume(TokenKind::RightArrow, "Expected '=>' after pattern")?;
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
    fn do_expression(&mut self) -> Result<Expression> {
        let location = self.previous().location.clone();
        let mut statements = Vec::new();

        while !self.check(TokenKind::End) && !self.is_at_end() {
            // Try to parse assignment first
            let current_pos = self.current;
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
                    // Not an assignment, backtrack
                    self.current = current_pos;
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

        // Parse optional type parameters
        let type_params = if self.match_token(TokenKind::Less) {
            self.parse_type_parameters()?
        } else {
            Vec::new()
        };

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

        // Parse where clause if present
        let where_clause = if self.match_token(TokenKind::Where) {
            Some(self.parse_where_clause()?)
        } else {
            None
        };

        // Parse function body
        self.consume(TokenKind::LeftBrace, "Expected '{' after function signature")?;
        let body = self.expression()?;
        self.consume(TokenKind::RightBrace, "Expected '}' after function body")?;

        // Detect captured variables
        let captured_vars = self.collect_captured_vars(&body, &parameters);

        Ok(Expression::FunctionLiteral(FunctionLiteralExpr {
            type_params,
            parameters,
            return_type,
            where_clause,
            body: Box::new(body),
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
            _ => {}
        }

        if let TokenKind::Identifier(name) = &self.peek().kind {
            let name = name.clone();
            let start_location = self.peek().location.clone();
            self.advance();

            // Check for generic variant: Constructor<Type>(payload)
            if self.match_token(TokenKind::Less) {
                // Parse type arguments
                let mut type_args = Vec::new();
                if !self.check(TokenKind::Greater) {
                    loop {
                        type_args.push(self.parse_type()?);
                        if !self.match_token(TokenKind::Comma) {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::Greater, "Expected '>' after type arguments")?;

                // Parse payload if present
                let payload = if self.match_token(TokenKind::LeftParen) {
                    let pat = self.pattern()?;
                    self.consume(TokenKind::RightParen, "Expected ')' after payload")?;
                    Some(Box::new(pat))
                } else {
                    None
                };

                Ok(Pattern::GenericVariant {
                    constructor: name,
                    type_args,
                    payload,
                })
            }
            // Check for regular variant: Constructor(payload)
            else if self.match_token(TokenKind::LeftParen) {
                let payload = if self.check(TokenKind::RightParen) {
                    None
                } else {
                    let pat = self.pattern()?;
                    Some(Box::new(pat))
                };
                self.consume(TokenKind::RightParen, "Expected ')' after variant payload")?;

                Ok(Pattern::Variant {
                    constructor: name,
                    payload,
                })
            } else {
            Ok(Pattern::Identifier(name))
            }
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
            parse_error(self.peek().location.clone(), message.to_string())
        }
    }
}
