# Silica Compiler Error Code Allocation

Per silica-error-code-scheme.jsonld and silica-specification.md §1.6.

## Phase Allocations

| Phase | Range | Category | Spec Section |
|-------|-------|----------|--------------|
| Lexer | E0001-E0999 | LexicalErrors | spec:§2 |
| Parser | E1000-E1999 | ParseErrors | spec:§3 |
| Type checker | E2000-E2999 | TypeErrors | spec:§6 |
| Effect checker | E3000-E3999 | EffectErrors | spec:§8 |
| SIR generator | E4000-E4049 | CodegenErrors (SIR) | - |
| Emitter | E4050-E4099 | CodegenErrors (emit) | - |
| Module | E5000-E5999 | ModuleErrors | spec:§11 |
| Internal | E9000-E9999 | InternalErrors | - |

## Specific Codes (from silica-error-code-scheme.jsonld)

### Lexer (E0001-E0999)
- E0000 LexerErrorDefault
- E0001 UnexpectedCharacter
- E0002 InvalidEscapeSequence
- E0003 UnterminatedStringLiteral
- E0004 UnterminatedCharacterLiteral
- E0005 InvalidIntegerLiteral
- E0006 UnterminatedBlockComment

### Parser (E1000-E1999)
- E1000 ParseErrorDefault
- E1001 ExpectedToken
- E1002 NestedFunctionDeclaration
- E1003 ExpectedIdentifier
- E1004 ExpectedType
- E1005 ExpectedExpression
- E1006 ExpectedMemorySpace
- E1007 WildcardRequiresTypeAnnotation
- E1008 UnsupportedSyntax

### Type checker (E2000-E2999)
- E2000 TypeErrorDefault
- E2001 TypeMismatch
- E2002 UndefinedType
- E2003 TypeUnificationFailure
- E2004 VariableShadowing
- E2005 TupleArityMismatch
- E2006 RecordFieldCountMismatch
- E2007 RecordFieldTypeMismatch
- E2008 FunctionReturnTypeMismatch
- E2009 MissingTraitImplementation
- E2010 TypeInferenceNotImplemented
- E2011 FunctionLiteralMissingEffectDeclaration
- E2012 TupleDecomposeBindingOverflow

### Effect checker (E3000-E3999)
- E3000 EffectErrorDefault
- E3001 MissingEffectCapability
- E3002 EffectNotActive
- E3003 EffectCompatibilityMismatch
- E3004 EffectPushFailure

### SIR generator (E4000-E4049)
- E4000 SIR default
- E4001-E4049 Reserved for SIR-specific errors

### Emitter (E4050-E4099)
- E4050 Emitter default
- E4051-E4099 Reserved for emitter-specific errors
