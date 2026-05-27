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

use crate::ast::*;
use crate::errors::{Result, CompilerError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// Represents a loaded module with its exports and dependencies
#[derive(Debug)]
pub struct LoadedModule {
    pub name: String,
    pub path: PathBuf,
    pub exports: Vec<ExportItem>,
    pub ast: Vec<Declaration>,
}

/// Module resolver handles loading and caching modules
pub struct ModuleResolver {
    search_paths: Vec<PathBuf>,
    loaded_modules: HashMap<String, LoadedModule>,
}

impl ModuleResolver {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            loaded_modules: HashMap::new(),
        }
    }

    /// Extract module name from file path (filename without .silica extension)
    pub fn module_name_from_path(&self, path: &Path) -> Result<String> {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| CompilerError::codegen_error(
                format!("Invalid module filename: {}", path.display())
            ))
    }

    /// Find module file by name in search paths
    pub fn find_module_file(&self, module_name: &str) -> Result<PathBuf> {
        let filename = format!("{}.silica", module_name);

        for search_path in &self.search_paths {
            let candidate = search_path.join(&filename);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(CompilerError::codegen_error(
            format!("Module '{}' not found in search paths: {:?}",
                module_name, self.search_paths)
        ))
    }

    /// Load a module by name, returning the loaded module
    pub fn load_module(&mut self, module_name: &str) -> Result<()> {
        // Check if already loaded
        if self.loaded_modules.contains_key(module_name) {
            return Ok(());
        }

        // Find the module file
        let module_path = self.find_module_file(module_name)?;

        // Read and parse the module file
        let source = fs::read_to_string(&module_path)
            .map_err(|e| CompilerError::codegen_error(
                format!("Failed to read module '{}': {}", module_name, e)
            ))?;

        // Parse the module (we'll need to integrate with the parser)
        let ast = self.parse_module_source(&source, module_name)?;

        // Extract exports
        let exports = self.extract_exports(&ast)?;

        // Create the loaded module
        let module = LoadedModule {
            name: module_name.to_string(),
            path: module_path,
            exports,
            ast,
        };

        // Cache it
        self.loaded_modules.insert(module_name.to_string(), module);
        Ok(())
    }

    /// Get a loaded module by name (must be loaded first)
    pub fn get_module(&self, module_name: &str) -> Option<&LoadedModule> {
        self.loaded_modules.get(module_name)
    }

    /// Get the count of currently loaded modules
    pub fn loaded_module_count(&self) -> usize {
        self.loaded_modules.len()
    }

    /// Parse module source code using the compiler's parser
    fn parse_module_source(&self, source: &str, module_name: &str) -> Result<Vec<Declaration>> {
        use crate::lexer::Lexer;
        use crate::parser::Parser;

        // Lexical analysis - use module:name format for clearer error messages when parsing fails
        let file_id = format!("module:{}", module_name);
        let mut lexer = Lexer::new(source.to_string(), file_id);
        let tokens = lexer.tokenize()?;

        // Parsing
        let mut parser = Parser::new(tokens);
        let program = parser.parse()?;

        Ok(program.declarations)
    }

    /// Extract export declarations from AST
    fn extract_exports(&self, ast: &[Declaration]) -> Result<Vec<ExportItem>> {
        let mut exports = Vec::new();

        for decl in ast {
            if let Declaration::Export(export_decl) = decl {
                exports.extend(export_decl.items.clone());
            }
        }

        Ok(exports)
    }

    /// Get all loaded modules
    pub fn loaded_modules(&self) -> &HashMap<String, LoadedModule> {
        &self.loaded_modules
    }
}

/// Symbol table for managing module-scoped symbols
#[derive(Debug, Clone)]
pub struct SymbolTable {
    pub modules: HashMap<String, HashMap<String, SymbolInfo>>,
}

#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub arity: u32,
    pub module: String,
    pub ty: crate::ast::Type, // Will be filled during type checking
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut symbol_table = Self {
            modules: HashMap::new(),
        };

        // Add built-in I/O functions
        let mut builtin_symbols = HashMap::new();

        // read_file(path: string) -> Result<string, string>
        builtin_symbols.insert("read_file".to_string(), SymbolInfo {
            name: "read_file".to_string(),
            arity: 1,
            module: "builtin".to_string(),
            ty: Type::Unit, // Will be properly typed during type checking
        });

        // write_file(path: string, content: string) -> Result<unit, string>
        builtin_symbols.insert("write_file".to_string(), SymbolInfo {
            name: "write_file".to_string(),
            arity: 2,
            module: "builtin".to_string(),
            ty: Type::Unit, // Will be properly typed during type checking
        });

        symbol_table.modules.insert("builtin".to_string(), builtin_symbols);

        symbol_table
    }

    /// Add symbols from a loaded module
    pub fn add_module_symbols(&mut self, module: &LoadedModule) -> Result<()> {
        let mut module_symbols = HashMap::new();

        for export in &module.exports {
            let symbol_info = SymbolInfo {
                name: export.name.clone(),
                arity: export.arity,
                module: module.name.clone(),
                ty: Type::Unit, // Placeholder - will be updated during type checking
            };
            module_symbols.insert(export.name.clone(), symbol_info);
        }

        self.modules.insert(module.name.clone(), module_symbols);
        Ok(())
    }

    /// Look up a symbol by name in a specific module
    pub fn lookup_symbol(&self, module_name: &str, symbol_name: &str) -> Option<&SymbolInfo> {
        self.modules.get(module_name)
            .and_then(|module_symbols| module_symbols.get(symbol_name))
    }

    /// Check if a symbol exists in a module
    pub fn symbol_exists(&self, module_name: &str, symbol_name: &str) -> bool {
        self.lookup_symbol(module_name, symbol_name).is_some()
    }
}
