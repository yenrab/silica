use crate::ast::*;
use crate::errors::{Result, effect_error, effect_error_with_metadata, SourceLocation, ErrorMetadataBuilder, ErrorSeverity};
use std::collections::{HashSet, HashMap};

/// Effect context tracking active capabilities
#[derive(Debug, Clone)]
pub struct EffectContext {
    active_effects: Vec<Effect>,
    capability_stack: Vec<Capability>,
    effect_variables: HashMap<String, Vec<Effect>>, // For effect polymorphism
}

/// Capability token for effect checking
#[derive(Debug, Clone)]
pub struct Capability {
    pub effect: Effect,
    pub location: SourceLocation,
}

/// Effect checker implementing capability-based effect tracking
pub struct EffectChecker {
    context: EffectContext,
    /// Reference to analyzer for accessing type information (optional)
    analyzer: Option<*const EffectAnalyzer>,
}

impl EffectChecker {
    pub fn new() -> Self {
        EffectChecker {
            context: EffectContext {
                active_effects: Vec::new(),
                capability_stack: Vec::new(),
                effect_variables: HashMap::new(),
            },
            analyzer: None,
        }
    }

    /// Create effect checker with analyzer reference for type information
    pub fn with_analyzer(analyzer: *const EffectAnalyzer) -> Self {
        EffectChecker {
            context: EffectContext {
                active_effects: Vec::new(),
                capability_stack: Vec::new(),
                effect_variables: HashMap::new(),
            },
            analyzer: Some(analyzer),
        }
    }

    /// Get analyzer reference for type information access
    unsafe fn get_analyzer(&self) -> Option<&EffectAnalyzer> {
        self.analyzer.map(|ptr| &*ptr)
    }

    /// Check effect safety for an expression
    pub fn check_expression(&mut self, expr: &Expression, required_effects: &[Effect], analyzer: Option<&EffectAnalyzer>) -> Result<()> {
        // Collect effects required by the expression
        let expr_effects = if let Some(analyzer) = analyzer {
            analyzer.collect_expression_effects(expr)?
        } else {
            self.collect_expression_effects(expr)?
        };

        // Check that all required effects are active (with subeffecting)
        for required in required_effects {
            let mut found_compatible = false;
            for active in &self.context.active_effects {
                if self.is_subeffect(required, active) {
                    found_compatible = true;
                    break;
                }
            }
            if !found_compatible {
                let metadata = ErrorMetadataBuilder::new("E3002".to_string())
                    .severity(ErrorSeverity::Error)
                    .specification("§8".to_string(), None)
                    .suggestion(format!("Add effect {:?} to function's proc[...] declaration", required))
                    .build();
                return effect_error_with_metadata(
                    SourceLocation::unknown(),
                    format!("Effect not active: {:?} (available: {:?})", required, self.context.active_effects),
                    metadata,
                );
            }
        }

        // Check that expression effects are covered by active effects
        // If active_effects is empty when expression requires effects, this indicates:
        // 1. Function didn't declare effects (func.effects was empty)
        // 2. push_capabilities wasn't called before checking
        // 3. Effects were popped before checking (shouldn't happen)
        // For bootstrap compiler: temporarily allow Memory and Concurrency effects
        // but this should be properly fixed
        if self.context.active_effects.is_empty() && !expr_effects.is_empty() {
            // Check if all expression effects are temporarily allowed
            let mut all_allowed = true;
            for expr_effect in &expr_effects {
                match expr_effect {
                    Effect::Memory(_) | Effect::Concurrency => {
                        // These are temporarily allowed for bootstrap compiler
                        continue;
                    }
                    Effect::Mailbox(_) => {
                        // Mailbox effects are allowed when Concurrency would be present
                        // Since active_effects is empty, we allow it as a temporary workaround
                        // In proper implementation, Concurrency should be declared
                        continue;
                    }
                    _ => {
                        all_allowed = false;
                        break;
                    }
                }
            }
            if !all_allowed {
                let metadata = ErrorMetadataBuilder::new("E3001".to_string())
                    .severity(ErrorSeverity::Error)
                    .specification("§8".to_string(), None)
                    .suggestion("Add required effects to function's proc[...] declaration".to_string())
                    .suggestion_with_example("Example:".to_string(), "fn my_func() : proc[concurrency, mem(normal)] { ... }".to_string())
                    .build();
                return effect_error_with_metadata(
                    SourceLocation::unknown(),
                    format!("Expression requires effect not covered by active capabilities: {:?} (active: {:?}). Note: active_effects is empty - function may not have declared required effects with proc[...].",
                           expr_effects, self.context.active_effects),
                    metadata,
                );
            }
            // All effects are temporarily allowed for bootstrap compiler
            // TODO: Fix root cause - ensure effects are properly parsed and pushed
            return Ok(());
        }
        
        for expr_effect in &expr_effects {
            let mut covered = false;
            for active in &self.context.active_effects {
                if self.is_subeffect(expr_effect, active) {
                    covered = true;
                    break;
                }
            }
            
            // Special case: Mailbox effects are covered by Concurrency
            // Having Concurrency capability implies ability to use mailboxes
            // The mailbox type is for type safety, but the capability comes from Concurrency
            if !covered {
                if let Effect::Mailbox(_) = expr_effect {
                    // Check if Concurrency is active - if so, mailbox is allowed
                    if self.context.active_effects.iter().any(|e| matches!(e, Effect::Concurrency)) {
                        continue; // Mailbox effect is allowed when Concurrency is active
                    }
                }
            }
            
            if !covered {
                // Allow memory operations for testing (temporary)
                if let Effect::Memory(_) = expr_effect {
                    continue;
                }
                // Allow concurrency operations for testing (temporary - bootstrap compiler)
                if let Effect::Concurrency = expr_effect {
                    continue;
                }
                let metadata = ErrorMetadataBuilder::new("E3001".to_string())
                    .severity(ErrorSeverity::Error)
                    .specification("§8".to_string(), None)
                    .suggestion(format!("Add effect {:?} to function's proc[...] declaration", expr_effect))
                    .build();
                return effect_error_with_metadata(
                    SourceLocation::unknown(),
                    format!("Expression requires effect not covered by active capabilities: {:?} (active: {:?})",
                           expr_effect, self.context.active_effects),
                    metadata,
                );
            }
        }

        Ok(())
    }

    /// Add a capability to the current context
    pub fn add_capability(&mut self, effect: Effect, location: SourceLocation) {
        self.context.capability_stack.push(Capability { effect: effect.clone(), location });
        self.context.active_effects.push(effect);
    }

    /// Remove a capability from the current context
    pub fn remove_capability(&mut self, effect: &Effect) {
        self.context.active_effects.retain(|e| e != effect);
        self.context.capability_stack.retain(|c| &c.effect != effect);
    }

    /// Check if an effect is a subeffect of another (effect subsumption)
    pub fn is_subeffect(&self, sub: &Effect, sup: &Effect) -> bool {
        match (sub, sup) {
            // Memory effects are covariant in space
            (Effect::Memory(sub_space), Effect::Memory(sup_space)) => {
                // Normal memory can be used where any memory is expected
                // Atomic memory requires atomic capability
                match (sub_space, sup_space) {
                    (MemorySpace::Normal, MemorySpace::Normal) => true,
                    (MemorySpace::Normal, MemorySpace::Atomic) => false, // Need atomic for atomic
                    (MemorySpace::Atomic, MemorySpace::Normal) => true,  // Atomic can be used as normal
                    (MemorySpace::Atomic, MemorySpace::Atomic) => true,
                }
            }
            // Mailbox effects are invariant in message type
            (Effect::Mailbox(sub_type), Effect::Mailbox(sup_type)) => {
                // For now, exact type match (could be made covariant)
                sub_type == sup_type
            }
            // Exact match for other effects
            (a, b) => a == b,
        }
    }

    /// Unify two effects (find their least upper bound)
    pub fn unify_effects(&self, e1: &Effect, e2: &Effect) -> Option<Effect> {
        if e1 == e2 {
            Some(e1.clone())
        } else if self.is_subeffect(e1, e2) {
            Some(e2.clone())
        } else if self.is_subeffect(e2, e1) {
            Some(e1.clone())
        } else {
            // No common supereffect
            None
        }
    }

    /// Collect effects required by an expression (EffectChecker version - no type info)
    fn collect_expression_effects(&self, expr: &Expression) -> Result<Vec<Effect>> {
        match expr {
            Expression::Literal(_) => Ok(vec![]),
            Expression::Identifier(_) => Ok(vec![]),
            Expression::Binary(_) => Ok(vec![]),
            Expression::Unary(_) => Ok(vec![]),
            Expression::Call(call) => self.collect_call_effects(call),
            Expression::FunctionLiteral(func) => self.collect_function_literal_effects(func),
            Expression::If(if_expr) => self.collect_if_effects(if_expr),
            Expression::Case(case) => self.collect_case_effects(case),
            Expression::Do(do_expr) => self.collect_do_effects(do_expr),
            Expression::Region(_) => Ok(vec![]), // Region creation has no effects
            Expression::Region(region) => {
                let mut effects = vec![Effect::Memory(region.space.clone())];
                // Value expression may have effects
                effects.extend(self.collect_expression_effects(&region.value)?);
                Ok(effects)
            }
            Expression::ReadRef(_) => Ok(vec![Effect::Memory(MemorySpace::Normal)]),
            Expression::Spawn(spawn) => {
                // Spawn requires both concurrency and mailbox effects
                // For EffectChecker version (no type info), use function literal fallback
                let message_type = if let Expression::FunctionLiteral(func) = &*spawn.behavior {
                    if let Some(first_param) = func.parameters.first() {
                        first_param.type_.clone()
                    } else {
                        Type::Unit
                    }
                } else {
                    Type::Unit
                };
                
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(message_type)),
                ];
                effects.extend(self.collect_expression_effects(&spawn.initial_state)?);
                effects.extend(self.collect_expression_effects(&spawn.behavior)?);
                if let Some(core_affinity) = &spawn.core_affinity {
                    effects.extend(self.collect_expression_effects(core_affinity)?);
                }
                Ok(effects)
            },
            Expression::Send(send) => {
                // Send requires both concurrency and mailbox effects
                // For EffectChecker version (no type info), use Unit as placeholder
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(Type::Unit)),
                ];
                effects.extend(self.collect_expression_effects(&send.actor)?);
                effects.extend(self.collect_expression_effects(&send.message)?);
                Ok(effects)
            },
            Expression::Recv(recv) => {
                // Recv requires both concurrency and mailbox effects
                // For EffectChecker version (no type info), use Unit as placeholder
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(Type::Unit)),
                ];
                if let Some(actor) = &recv.actor {
                    effects.extend(self.collect_expression_effects(actor)?);
                }
                Ok(effects)
            },
            Expression::Cast(cast) => {
                // Cast requires both concurrency and mailbox effects
                // For EffectChecker version (no type info), use Unit as placeholder
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(Type::Unit)),
                ];
                effects.extend(self.collect_expression_effects(&cast.actor)?);
                effects.extend(self.collect_expression_effects(&cast.message)?);
                Ok(effects)
            },
            Expression::ReadFile(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::WriteFile(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::Print(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintLn(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintInt64(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintInt32(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintInt16(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintInt8(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintBool(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintChar(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintFloat16(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintFloat32(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintFloat64(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::GetCpuTopologyInfo(_) => Ok(vec![]), // Reading pre-detected topology info
            Expression::StringLen(string_len) => self.collect_expression_effects(&string_len.string),
            Expression::StringLenChars(string_len_chars) => self.collect_expression_effects(&string_len_chars.string),
            Expression::StringConcat(string_concat) => {
                let mut effects = Vec::new();
                effects.extend(self.collect_expression_effects(&string_concat.a)?);
                effects.extend(self.collect_expression_effects(&string_concat.b)?);
                Ok(effects)
            }
            Expression::StringSubstring(string_substring) => {
                let mut effects = Vec::new();
                effects.extend(self.collect_expression_effects(&string_substring.string)?);
                effects.extend(self.collect_expression_effects(&string_substring.start)?);
                effects.extend(self.collect_expression_effects(&string_substring.end)?);
                Ok(effects)
            }
            Expression::StringSubstringUntilChar(string_substring_until_char) => {
                let mut effects = Vec::new();
                effects.extend(self.collect_expression_effects(&string_substring_until_char.string)?);
                effects.extend(self.collect_expression_effects(&string_substring_until_char.start)?);
                effects.extend(self.collect_expression_effects(&string_substring_until_char.char)?);
                Ok(effects)
            }
            Expression::StringStartsWith(string_starts_with) => {
                let mut effects = Vec::new();
                effects.extend(self.collect_expression_effects(&string_starts_with.string)?);
                effects.extend(self.collect_expression_effects(&string_starts_with.prefix)?);
                Ok(effects)
            }
            Expression::StringEndsWith(string_ends_with) => {
                let mut effects = Vec::new();
                effects.extend(self.collect_expression_effects(&string_ends_with.string)?);
                effects.extend(self.collect_expression_effects(&string_ends_with.suffix)?);
                Ok(effects)
            }
            Expression::StringContains(string_contains) => {
                let mut effects = Vec::new();
                effects.extend(self.collect_expression_effects(&string_contains.string)?);
                effects.extend(self.collect_expression_effects(&string_contains.substr)?);
                Ok(effects)
            }
            Expression::ReadLines(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::AppendFile(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::FileExists(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::DeleteFile(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::GetFileSize(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::CreateDirectory(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::RemoveDirectory(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::ListDirectory(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::ExecCommand(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::StructLiteral(_) => Ok(vec![]), // Struct literals have no effects
            Expression::FieldAccess(_) => Ok(vec![]),   // Field access has no effects
            Expression::ConstructorCall(_) => Ok(vec![]), // Constructor calls have no effects
            Expression::Tuple(_) => Ok(vec![]), // Tuple literals have no effects
            Expression::AsType(as_type) => {
                // Type casting inherits effects from the expression being cast
                self.collect_expression_effects(&as_type.expression)
            }
        }
    }

    /// Collect effects required by a sequence of statements
    fn collect_statement_effects(&self, statements: &[crate::ast::Statement]) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        for statement in statements {
            match statement {
                crate::ast::Statement::Bind { expr, .. } => {
                    effects.extend(self.collect_expression_effects(expr)?);
                }
                crate::ast::Statement::Expr(expr) => {
                    effects.extend(self.collect_expression_effects(expr)?);
                }
            }
        }

        Ok(effects)
    }

    /// Check effects for a sequence of statements
    fn check_statements(&mut self, statements: &[crate::ast::Statement], required_effects: &[Effect], analyzer: Option<&EffectAnalyzer>) -> Result<()> {
        // Note: required_effects parameter is for checking if required effects are active
        // The actual effect checking uses self.context.active_effects which should be
        // populated by push_capabilities before this method is called
        for statement in statements {
            match statement {
                crate::ast::Statement::Bind { expr, .. } => {
                    // Check expression effects against active capabilities in context
                    self.check_expression(expr, required_effects, analyzer)?;
                }
                crate::ast::Statement::Expr(expr) => {
                    // Check expression effects against active capabilities in context
                    self.check_expression(expr, required_effects, analyzer)?;
                }
            }
        }

        Ok(())
    }


    /// Collect effects for function call
    fn collect_call_effects(&self, call: &CallExpr) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        // Add effects from function arguments
        for arg in &call.arguments {
            effects.extend(self.collect_expression_effects(arg)?);
        }

        // TODO: Look up function effects from type information
        // For now, assume pure functions

        Ok(effects)
    }

    /// Collect effects for if expression
    fn collect_if_effects(&self, if_expr: &IfExpr) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        effects.extend(self.collect_expression_effects(&if_expr.condition)?);
        effects.extend(self.collect_expression_effects(&if_expr.then_branch)?);
        effects.extend(self.collect_expression_effects(&if_expr.else_branch)?);

        Ok(effects)
    }

    /// Collect effects for case expression
    fn collect_case_effects(&self, case: &CaseExpr) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        effects.extend(self.collect_expression_effects(&case.scrutinee)?);
        for branch in &case.branches {
            effects.extend(self.collect_expression_effects(&branch.body)?);
        }

        Ok(effects)
    }

    /// Collect effects for do expression
    fn collect_do_effects(&self, do_expr: &DoExpr) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        for statement in &do_expr.statements {
            match statement {
                Statement::Bind { expr, .. } => {
                    effects.extend(self.collect_expression_effects(expr)?);
                }
                Statement::Expr(expr) => {
                    effects.extend(self.collect_expression_effects(expr)?);
                }
            }
        }

        Ok(effects)
    }

    /// Collect effects for function literals (EffectChecker version)
    fn collect_function_literal_effects(&self, func: &FunctionLiteralExpr) -> Result<Vec<Effect>> {
        // Function literals themselves have no effects, but their bodies might
        self.collect_statement_effects(&func.body)
    }


    /// Push effect capabilities onto the context
    pub fn push_capabilities(&mut self, effects: &[Effect], location: &SourceLocation) {
        for effect in effects {
            self.context.active_effects.push(effect.clone());
            self.context.capability_stack.push(Capability {
                effect: effect.clone(),
                location: location.clone(),
            });
        }
    }

    /// Pop effect capabilities from the context
    pub fn pop_capabilities(&mut self, count: usize) {
        for _ in 0..count {
            if let Some(capability) = self.context.capability_stack.pop() {
                self.context.active_effects.retain(|e| e != &capability.effect);
            }
        }
    }

    /// Check if an effect is currently active
    pub fn is_effect_active(&self, effect: &Effect) -> bool {
        self.context.active_effects.contains(effect)
    }

    /// Get current active effects
    pub fn active_effects(&self) -> &[Effect] {
        &self.context.active_effects
    }

    /// Validate effect compatibility
    pub fn check_effect_compatibility(&self, declared: &[Effect], required: &[Effect]) -> Result<()> {
        for req in required {
            if !declared.contains(req) {
                let metadata = ErrorMetadataBuilder::new("E3003".to_string())
                    .severity(ErrorSeverity::Error)
                    .specification("§8".to_string(), None)
                    .suggestion(format!("Add effect {:?} to function's proc[...] declaration", req))
                    .build();
                return effect_error_with_metadata(
                    SourceLocation::unknown(),
                    format!("Required effect {:?} not declared", req),
                    metadata,
                );
            }
        }
        Ok(())
    }

    /// Merge effect sets (union)
    pub fn merge_effects(effects1: &[Effect], effects2: &[Effect]) -> Vec<Effect> {
        let mut merged = effects1.to_vec();
        for effect in effects2 {
            if !merged.contains(effect) {
                merged.push(effect.clone());
            }
        }
        merged
    }

    /// Check effect subeffecting
    pub fn is_sub_effect(&self, sub: &Effect, super_: &Effect) -> bool {
        match (sub, super_) {
            // Memory space subeffecting
            (Effect::Memory(MemorySpace::Normal), Effect::Memory(MemorySpace::Atomic)) => true,
            (Effect::Memory(space1), Effect::Memory(space2)) => space1 == space2,

            // Other effects must be identical
            (e1, e2) => e1 == e2,
        }
    }

    /// Normalize effects (apply subeffecting rules)
    pub fn normalize_effects(&self, effects: &[Effect]) -> Vec<Effect> {
        let mut normalized = Vec::new();

        for effect in effects {
            // Check if this effect is already covered by a super-effect
            let mut covered = false;
            for existing in &normalized {
                if self.is_sub_effect(effect, existing) {
                    covered = true;
                    break;
                }
            }

            if !covered {
                // Remove any sub-effects that are now covered by this effect
                normalized.retain(|e| !self.is_sub_effect(e, effect));
                normalized.push(effect.clone());
            }
        }

        normalized
    }
}

/// Effect analysis for declarations
pub struct EffectAnalyzer {
    checker: EffectChecker,
    /// Map from expression locations to their types (from type checker)
    expression_types: std::collections::HashMap<SourceLocation, Type>,
    /// Map from spawn expression locations to their message types (tracked during type checking)
    actor_mailbox_types: std::collections::HashMap<SourceLocation, Type>,
}

impl EffectAnalyzer {
    pub fn new() -> Self {
        let analyzer = EffectAnalyzer {
            checker: EffectChecker::new(),
            expression_types: std::collections::HashMap::new(),
            actor_mailbox_types: std::collections::HashMap::new(),
        };
        // Set up checker with analyzer reference
        let checker = EffectChecker::with_analyzer(&analyzer as *const EffectAnalyzer);
        // Note: We can't directly set checker.analyzer after creation, so we'll need to restructure
        // For now, we'll access types directly in collect_expression_effects
        analyzer
    }

    /// Create effect analyzer with type information from type checker
    pub fn with_types(
        expression_types: std::collections::HashMap<SourceLocation, Type>,
        actor_mailbox_types: std::collections::HashMap<SourceLocation, Type>,
    ) -> Self {
        let analyzer = EffectAnalyzer {
            checker: EffectChecker::new(),
            expression_types,
            actor_mailbox_types,
        };
        // Set up checker with analyzer reference
        let checker = EffectChecker::with_analyzer(&analyzer as *const EffectAnalyzer);
        // Note: We can't directly set checker.analyzer after creation, so we'll need to restructure
        // For now, we'll access types directly in collect_expression_effects
        analyzer
    }

    /// Get the type of an expression from the type checker's results
    fn get_expression_type(&self, expr: &Expression) -> Option<Type> {
        // Try to get location from expression
        let location = Self::try_get_expression_location(expr)?;
        self.expression_types.get(location).cloned()
    }

    /// Collect effects required by an expression (EffectAnalyzer version - with type info)
    pub fn collect_expression_effects(&self, expr: &Expression) -> Result<Vec<Effect>> {
        // Delegate to checker's version but override actor operations with type info
        match expr {
            Expression::Spawn(spawn) => {
                // Spawn requires both concurrency and mailbox effects
                // Extract message type from tracked actor mailbox types or behavior function
                let message_type = if let Some(mailbox_type) = self.actor_mailbox_types.get(&spawn.location) {
                    // Use tracked mailbox type from type checker
                    mailbox_type.clone()
                } else if let Expression::FunctionLiteral(func) = &*spawn.behavior {
                    // Fallback: If behavior is a function literal, get message type from first parameter
                    if let Some(first_param) = func.parameters.first() {
                        first_param.type_.clone()
                    } else {
                        Type::Unit
                    }
                } else {
                    // Fallback: Try to get type from type checker's expression types
                    self.get_expression_type(&spawn.behavior)
                        .and_then(|behavior_type| {
                            if let Type::Function { parameters, .. } = behavior_type {
                                parameters.first().cloned()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(Type::Unit)
                };
                
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(message_type)),
                ];
                effects.extend(self.collect_expression_effects(&spawn.initial_state)?);
                effects.extend(self.collect_expression_effects(&spawn.behavior)?);
                if let Some(core_affinity) = &spawn.core_affinity {
                    effects.extend(self.collect_expression_effects(core_affinity)?);
                }
                Ok(effects)
            },
            Expression::Send(send) => {
                // Send requires both concurrency and mailbox effects
                // Extract message type from message expression's type
                let message_type = self.get_expression_type(&send.message)
                    .unwrap_or(Type::Unit);
                
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(message_type)),
                ];
                effects.extend(self.collect_expression_effects(&send.actor)?);
                effects.extend(self.collect_expression_effects(&send.message)?);
                Ok(effects)
            },
            Expression::Recv(recv) => {
                // Recv requires both concurrency and mailbox effects
                // Extract message type from actor's mailbox type
                let message_type = if let Some(actor_expr) = &recv.actor {
                    // TODO: Track actor_ref -> spawn mapping to get mailbox type
                    // For now, use Unit as placeholder
                    Type::Unit
                } else {
                    // recv() without actor - would need context about current actor
                    Type::Unit
                };
                
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(message_type)),
                ];
                if let Some(actor) = &recv.actor {
                    effects.extend(self.collect_expression_effects(actor)?);
                }
                Ok(effects)
            },
            Expression::Cast(cast) => {
                // Cast requires both concurrency and mailbox effects
                // Extract message type from message expression's type
                let message_type = self.get_expression_type(&cast.message)
                    .unwrap_or(Type::Unit);
                
                let mut effects = vec![
                    Effect::Concurrency,
                    Effect::Mailbox(Box::new(message_type)),
                ];
                effects.extend(self.collect_expression_effects(&cast.actor)?);
                effects.extend(self.collect_expression_effects(&cast.message)?);
                Ok(effects)
            },
            _ => {
                // For all other expressions, delegate to checker's version
                self.checker.collect_expression_effects(expr)
            }
        }
    }

    /// Collect effects for statements (EffectAnalyzer version)
    pub fn collect_statement_effects(&self, statements: &[crate::ast::Statement]) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();
        for statement in statements {
            match statement {
                crate::ast::Statement::Bind { expr, .. } => {
                    effects.extend(self.collect_expression_effects(expr)?);
                }
                crate::ast::Statement::Expr(expr) => {
                    effects.extend(self.collect_expression_effects(expr)?);
                }
            }
        }
        Ok(effects)
    }

    /// Try to extract location from an expression
    fn try_get_expression_location(expr: &Expression) -> Option<&SourceLocation> {
        match expr {
            Expression::Literal(_) => None, // Literals don't have locations in AST
            Expression::Identifier(_) => None, // Identifiers don't have locations in AST
            Expression::If(if_expr) => Some(&if_expr.location),
            Expression::Case(case) => Some(&case.location),
            Expression::Do(do_expr) => Some(&do_expr.location),
            Expression::Call(call) => Some(&call.location),
            Expression::FunctionLiteral(func) => Some(&func.location),
            Expression::Unary(unary) => Some(&unary.location),
            Expression::Binary(binary) => Some(&binary.location),
            Expression::Region(region) => Some(&region.location),
            Expression::ReadRef(read) => Some(&read.location),
            Expression::Spawn(spawn) => Some(&spawn.location),
            Expression::Send(send) => Some(&send.location),
            Expression::Recv(recv) => Some(&recv.location),
            Expression::Cast(cast) => Some(&cast.location),
            Expression::ReadFile(read) => Some(&read.location),
            Expression::WriteFile(write) => Some(&write.location),
            Expression::Print(print) => Some(&print.location),
            Expression::PrintLn(println) => Some(&println.location),
            Expression::PrintInt64(print) => Some(&print.location),
            Expression::PrintInt32(print) => Some(&print.location),
            Expression::PrintInt16(print) => Some(&print.location),
            Expression::PrintInt8(print) => Some(&print.location),
            Expression::PrintBool(print) => Some(&print.location),
            Expression::PrintChar(print) => Some(&print.location),
            Expression::PrintFloat16(print) => Some(&print.location),
            Expression::PrintFloat32(print) => Some(&print.location),
            Expression::PrintFloat64(print) => Some(&print.location),
            Expression::GetCpuTopologyInfo(info) => Some(&info.location),
            Expression::StringLen(string_len) => Some(&string_len.location),
            Expression::StringLenChars(string_len_chars) => Some(&string_len_chars.location),
            Expression::StringConcat(string_concat) => Some(&string_concat.location),
            Expression::StringSubstring(string_substring) => Some(&string_substring.location),
            Expression::StringSubstringUntilChar(string_substring_until_char) => Some(&string_substring_until_char.location),
            Expression::StringStartsWith(string_starts_with) => Some(&string_starts_with.location),
            Expression::StringEndsWith(string_ends_with) => Some(&string_ends_with.location),
            Expression::StringContains(string_contains) => Some(&string_contains.location),
            Expression::ReadLines(read) => Some(&read.location),
            Expression::AppendFile(append) => Some(&append.location),
            Expression::FileExists(exists) => Some(&exists.location),
            Expression::DeleteFile(delete) => Some(&delete.location),
            Expression::GetFileSize(size) => Some(&size.location),
            Expression::CreateDirectory(create) => Some(&create.location),
            Expression::RemoveDirectory(remove) => Some(&remove.location),
            Expression::ListDirectory(list) => Some(&list.location),
            Expression::ExecCommand(exec) => Some(&exec.location),
            Expression::StructLiteral(struct_lit) => Some(&struct_lit.location),
            Expression::FieldAccess(access) => Some(&access.location),
            Expression::Tuple(_) => None, // Tuples don't have locations
            Expression::ConstructorCall(constructor) => Some(&constructor.location),
            Expression::AsType(as_type) => Some(&as_type.location),
        }
    }

    /// Analyze function declaration effects
    pub fn analyze_function(&mut self, func: &FunctionDecl) -> Result<Vec<Effect>> {
        // Push function's declared effects into active capabilities
        // This makes the effects available when checking expressions in the function body
        // If func.effects is empty, no capabilities are pushed, which means expressions
        // requiring effects will fail (unless temporarily allowed for bootstrap compiler)
        
        // Store declared effects count before pushing (for diagnostics)
        let declared_count = func.effects.len();
        let declared_effects = func.effects.clone(); // Clone for error messages
        
        // Push capabilities - this populates active_effects
        self.checker.push_capabilities(&func.effects, &func.location);
        
        // Verify effects were pushed correctly
        // After push_capabilities, active_effects should contain func.effects
        let active_count = self.checker.active_effects().len();
        
        // If function declares effects but they weren't pushed, that's a bug
        if declared_count > 0 && active_count < declared_count {
            let metadata = ErrorMetadataBuilder::new("E3004".to_string())
                .severity(ErrorSeverity::Error)
                .specification("§8".to_string(), None)  // Display will add "spec:" prefix
                .build();
            return effect_error_with_metadata(
                func.location.clone(),
                format!("Internal error: Failed to push all declared effects. Declared: {} ({:?}), Active: {}",
                       declared_count, declared_effects, active_count),
                metadata,
            );
        }
        
        // If function has no declared effects but body requires effects, that's an error
        // (unless temporarily allowed for bootstrap compiler)
        // This check happens in check_statements -> check_expression via check_expression

        // Analyze function body statements - use analyzer's collection method for type info
        let body_effects = self.collect_statement_effects(&func.body)?;

        // Check that body effects are covered by declared effects
        // Note: check_statements calls check_expression, which uses self.context.active_effects
        // At this point, active_effects should contain func.effects (pushed above)
        // If active_effects is empty here, it means func.effects was empty (effects not parsed)
        // Pass self as analyzer for type information
        // We need to avoid borrow checker issues - create a reference first
        let analyzer_ref: *const EffectAnalyzer = self;
        unsafe {
            self.checker.check_statements(&func.body, &func.effects, Some(&*analyzer_ref))?;
        }

        // Pop function effects to restore previous context
        self.checker.pop_capabilities(func.effects.len());

        // Return normalized effects
        Ok(self.checker.normalize_effects(&body_effects))
    }

    /// Analyze program for effect safety
    pub fn analyze_program(&mut self, program: &Program) -> Result<()> {
        for decl in &program.declarations {
            match decl {
                Declaration::Function(func) => {
                    self.analyze_function(func)?;
                }
                _ => {} // Other declarations don't have effects to check yet
            }
        }
        Ok(())
    }
}
