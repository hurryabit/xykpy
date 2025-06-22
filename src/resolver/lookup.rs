use std::collections::HashMap;

use ast::HasNodeIndex;
use text_size::Ranged;

use crate::{
    HasId,
    error::{ErrorsBuilder, TypeError},
    scope::{ScopeId, ScopeTable},
    symbol::{DeclOrDefn, Symbol, SymbolId, SymbolKind, SymbolTable},
};

pub struct ScopeLookup<'m>(HashMap<&'m ast::name::Name, SymbolId>);

pub(super) struct ScopeLoopkupBuilder<'m, 's> {
    symbols: &'s mut SymbolTable,
    scopes: &'s mut ScopeTable,
    errors: &'s mut ErrorsBuilder,
    scope_id: ScopeId,
    lookup: HashMap<&'m ast::name::Name, SymbolId>,
}

impl<'m, 's> ScopeLoopkupBuilder<'m, 's> {
    pub(super) fn new(
        symbols: &'s mut SymbolTable,
        scopes: &'s mut ScopeTable,
        errors: &'s mut ErrorsBuilder,
        scope_id: ScopeId,
    ) -> Self {
        let lookup = HashMap::new();
        Self {
            symbols,
            scopes,
            errors,
            scope_id,
            lookup,
        }
    }

    pub(super) fn build(self) -> ScopeLookup<'m> {
        ScopeLookup(self.lookup)
    }

    fn make_symbol(
        &self,
        kind: SymbolKind,
        name: impl HasNodeIndex + Ranged,
        decl_defn: DeclOrDefn<impl HasNodeIndex>,
    ) -> Symbol {
        Symbol::make(kind, self.scope_id, &name, decl_defn)
    }

    pub(super) fn add_symbol(
        &mut self,
        kind: SymbolKind,
        name: &'m (impl HasId + HasNodeIndex + Ranged),
        decl_defn: DeclOrDefn<impl HasNodeIndex>,
    ) {
        use std::collections::hash_map::Entry;
        let symbol = Symbol::make(kind, self.scope_id, name, decl_defn);
        match self.lookup.entry(name.id()) {
            Entry::Vacant(entry) => {
                let id = self.symbols.insert(symbol);
                self.scopes.add_symbol(self.scope_id, id);
                entry.insert(id);
            }
            Entry::Occupied(entry) => {
                let id = *entry.get();
                let previous = self.symbols.get(id);
                let (merged, conflict) = previous.merge(&symbol);
                if conflict {
                    let error = TypeError::new(
                        symbol.name_range,
                        format!(
                            "{} definition conflicts with earlier {} definition at {:?}",
                            symbol.kind, previous.kind, previous.name_range,
                        ),
                    );
                    self.errors.add(error);
                }
                if let Some(merged) = merged {
                    *self.symbols.get_mut(id) = merged;
                }
            }
        }
    }

    pub(super) fn add_stmt(&mut self, stmt: &'m ast::Stmt) {
        use DeclOrDefn::*;
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                self.add_symbol(SymbolKind::Class, &class_def.name, Decl(class_def));
            }
            ast::Stmt::TypeAlias(alias_def) => match &*alias_def.name {
                ast::Expr::Name(name) => {
                    self.add_symbol(SymbolKind::Alias, name, Decl(alias_def));
                }
                _ => {
                    unreachable!("The grammar only allows `type A = ...` and `type A[...] = ...`.");
                }
            },
            ast::Stmt::Assign(assign) => match &assign.targets[..] {
                [target] => match target {
                    ast::Expr::Name(name) => {
                        self.add_symbol(SymbolKind::Variable, name, Defn(assign));
                    }
                    _ => self.errors.add(TypeError::new(
                        target.range(),
                        "only name targets are supported",
                    )),
                },

                _ => {
                    self.errors.add(TypeError::new(
                        assign.range,
                        "only single target assignments are support",
                    ));
                }
            },
            ast::Stmt::AnnAssign(assign) => match &*assign.target {
                ast::Expr::Name(name) => {
                    if assign.value.is_some() {
                        self.add_symbol(SymbolKind::Variable, name, DeclAndDefn(assign))
                    } else {
                        self.add_symbol(SymbolKind::Variable, name, Decl(assign))
                    }
                }
                _ => self.errors.add(TypeError::new(
                    assign.target.range(),
                    "only name targets are supported",
                )),
            },
            ast::Stmt::FunctionDef(func_def) => {
                self.add_symbol(SymbolKind::Function, &func_def.name, Decl(func_def));
            }
            ast::Stmt::Nonlocal(nonlocal) => {
                for name in &nonlocal.names {
                    self.add_symbol(SymbolKind::Nonlocal, name, Decl(nonlocal));
                }
            }
            _ => {}
        }
    }

    pub(super) fn add_block(&mut self, stmts: &'m Vec<ast::Stmt>) {
        for stmt in stmts {
            self.add_stmt(stmt);
        }
    }
}
