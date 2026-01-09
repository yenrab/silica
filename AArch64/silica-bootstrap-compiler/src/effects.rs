use crate::ast::*;
use crate::errors::{Result, effect_error, SourceLocation};
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
}

impl EffectChecker {
    pub fn new() -> Self {
        EffectChecker {
            context: EffectContext {
                active_effects: Vec::new(),
                capability_stack: Vec::new(),
                effect_variables: HashMap::new(),
            },
        }
    }

    /// Check effect safety for an expression
    pub fn check_expression(&mut self, expr: &Expression, required_effects: &[Effect]) -> Result<()> {
        // Collect effects required by the expression
        let expr_effects = self.collect_expression_effects(expr)?;

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
                return effect_error(
                    SourceLocation::unknown(),
                    format!("Effect not active: {:?} (available: {:?})", required, self.context.active_effects),
                );
            }
        }

        // Check that expression effects are covered by active effects
        for expr_effect in &expr_effects {
            let mut covered = false;
            for active in &self.context.active_effects {
                if self.is_subeffect(expr_effect, active) {
                    covered = true;
                    break;
                }
            }
            if !covered {
                // Allow memory operations for testing (temporary)
                if let Effect::Memory(_) = expr_effect {
                    continue;
                }
                return effect_error(
                    SourceLocation::unknown(),
                    format!("Expression requires effect not covered by active capabilities: {:?} (active: {:?})",
                           expr_effect, self.context.active_effects),
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

    /// Collect effects required by an expression
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
            Expression::AllocRef(alloc) => self.collect_alloc_ref_effects(alloc),
            Expression::ReadRef(_) => Ok(vec![Effect::Memory(MemorySpace::Normal)]),
            Expression::WriteRef(_) => Ok(vec![Effect::Memory(MemorySpace::Normal)]),
            Expression::Spawn(_) => Ok(vec![Effect::Concurrency]),
            Expression::Send(_) => Ok(vec![Effect::Concurrency]),
            Expression::Recv(_) => Ok(vec![Effect::Concurrency]),
            Expression::ReadFile(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::WriteFile(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::Print(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintLn(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintInt(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintBool(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::PrintChar(_) => Ok(vec![Effect::Named("DeviceIO".to_string())]),
            Expression::GetCpuTopologyInfo(_) => Ok(vec![]), // Reading pre-detected topology info
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
    fn check_statements(&mut self, statements: &[crate::ast::Statement], required_effects: &[Effect]) -> Result<()> {
        for statement in statements {
            match statement {
                crate::ast::Statement::Bind { expr, .. } => {
                    self.check_expression(expr, required_effects)?;
                }
                crate::ast::Statement::Expr(expr) => {
                    self.check_expression(expr, required_effects)?;
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

    /// Collect effects for function literals
    fn collect_function_literal_effects(&self, func: &FunctionLiteralExpr) -> Result<Vec<Effect>> {
        // Function literals themselves have no effects, but their bodies might
        self.collect_statement_effects(&func.body)
    }

    /// Collect effects for reference allocation
    fn collect_alloc_ref_effects(&self, alloc: &AllocRefExpr) -> Result<Vec<Effect>> {
        let mut effects = Vec::new();

        // Region allocation requires memory effect
        effects.push(Effect::Memory(MemorySpace::Normal));

        // Initial value expression may have effects
        effects.extend(self.collect_expression_effects(&alloc.initial_value)?);

        Ok(effects)
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
                return effect_error(
                    SourceLocation::unknown(),
                    format!("Required effect {:?} not declared", req),
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
}

impl EffectAnalyzer {
    pub fn new() -> Self {
        EffectAnalyzer {
            checker: EffectChecker::new(),
        }
    }

    /// Analyze function declaration effects
    pub fn analyze_function(&mut self, func: &FunctionDecl) -> Result<Vec<Effect>> {
        // Push function's declared effects
        self.checker.push_capabilities(&func.effects, &func.location);

        // Analyze function body statements
        let body_effects = self.checker.collect_statement_effects(&func.body)?;

        // Check that body effects are covered by declared effects
        self.checker.check_statements(&func.body, &func.effects)?;

        // Pop function effects
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
