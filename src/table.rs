#![allow(dead_code)]
use std::collections::HashMap;

use ast::HasNodeIndex;
use text_size::Ranged;

use crate::error::{Errors, ErrorsBuilder, Outcome, TypeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Class,
    Alias,
    Variable,
    Function,
    Nonlocal,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            SymbolKind::Class => "class",
            SymbolKind::Alias => "type alias",
            SymbolKind::Variable => "variable",
            SymbolKind::Function => "function",
            SymbolKind::Nonlocal => "nonlocal",
        };
        f.write_str(text)
    }
}

fn no_node_index() -> ast::NodeIndex {
    ast::AtomicNodeIndex::dummy().load()
}

#[derive(Clone, Copy, Debug)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: ast::NodeIndex,
    pub name_range: text_size::TextRange,
    pub decl: ast::NodeIndex,
    pub defn: ast::NodeIndex,
}

static_assertions::const_assert_eq!(std::mem::size_of::<Symbol>(), 24);

enum DeclOrDefn<T> {
    Decl(T),
    Defn(T),
}

impl Symbol {
    fn is_decl(&self) -> bool {
        self.decl != no_node_index()
    }

    fn is_defn(&self) -> bool {
        self.decl != no_node_index()
    }

    // Returns whether the two symbols conflict.
    fn merge(&self, later: &Symbol) -> (Option<Symbol>, bool) {
        use SymbolKind::*;
        match (self.kind, later.kind) {
            (Variable, Variable) => {
                let conflict = self.is_decl() && later.is_decl();
                let decl = if self.is_decl() || !later.is_decl() {
                    self
                } else {
                    later
                };
                let defn = ast::NodeIndex::min(self.defn, later.defn);
                let merged = Symbol { defn, ..*decl };
                (Some(merged), conflict)
            }
            (Variable, Nonlocal) => (Some(*later), self.is_decl()),
            (Nonlocal, Variable) => (None, self.is_decl()),
            _ => (None, true),
        }
    }

    fn make(
        kind: SymbolKind,
        name: impl HasNodeIndex + Ranged,
        decl_defn: DeclOrDefn<impl HasNodeIndex>,
    ) -> Self {
        let (decl, defn) = match decl_defn {
            DeclOrDefn::Decl(decl) => (decl.node_index().load(), no_node_index()),
            DeclOrDefn::Defn(defn) => (no_node_index(), defn.node_index().load()),
        };
        Self {
            kind,
            name: name.node_index().load(),
            name_range: name.range(),
            decl,
            defn,
        }
    }
}

#[derive(Clone, Copy)]
pub struct SymbolId(u32);

pub struct SymbolTable(Vec<Symbol>);

pub struct ScopeTable<'m>(HashMap<&'m ast::name::Name, SymbolId>);

struct ResolutionTable(HashMap<ast::NodeIndex, SymbolId>);

impl SymbolTable {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn insert(&mut self, symbol: Symbol) -> SymbolId {
        let id = self.0.len().try_into().expect("More than 4G symbols? Wow!");
        self.0.push(symbol);
        SymbolId(id)
    }

    pub fn get(&self, id: SymbolId) -> &Symbol {
        &self.0[id.0 as usize]
    }

    pub fn get_mut(&mut self, id: SymbolId) -> &mut Symbol {
        &mut self.0[id.0 as usize]
    }
}

impl<'m> ScopeTable<'m> {
    fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn entries(&self) -> &HashMap<&'m ast::name::Name, SymbolId> {
        &self.0
    }

    fn insert(
        &mut self,
        table: &mut SymbolTable,
        name: &'m ast::name::Name,
        symbol: Symbol,
    ) -> Errors {
        use std::collections::hash_map::Entry;
        match self.0.entry(name) {
            Entry::Vacant(entry) => {
                let id = table.insert(symbol);
                entry.insert(id);
                Errors::AllGood
            }
            Entry::Occupied(entry) => {
                let id = *entry.get();
                let previous = table.get(id);
                let (merged, conflict) = previous.merge(&symbol);
                let errors = if conflict {
                    TypeError::new(
                        symbol.name_range,
                        format!(
                            "{} definition conflicts with earlier {} definition at {:?}",
                            symbol.kind, previous.kind, previous.name_range,
                        ),
                    )
                    .into()
                } else {
                    Errors::AllGood
                };
                if let Some(merged) = merged {
                    *table.get_mut(id) = merged;
                }
                errors
            }
        }
    }
}

pub fn block_scope<'m>(
    symbols: &mut SymbolTable,
    stmts: &'m Vec<ast::Stmt>,
) -> Outcome<ScopeTable<'m>> {
    use DeclOrDefn::*;
    let mut scope = ScopeTable::new();
    let mut errors = ErrorsBuilder::new();
    for stmt in stmts {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                let name = &class_def.name;
                let symbol = Symbol::make(SymbolKind::Class, name, Decl(class_def));
                errors.add(scope.insert(symbols, &name.id, symbol));
            }
            ast::Stmt::TypeAlias(alias_def) => match &*alias_def.name {
                ast::Expr::Name(name) => {
                    let symbol = Symbol::make(SymbolKind::Alias, name, Decl(alias_def));
                    errors.add(scope.insert(symbols, &name.id, symbol));
                }
                _ => {
                    unreachable!("The grammar only allows `type A = ...` and `type A[...] = ...`.");
                }
            },
            ast::Stmt::Assign(assign) => match &assign.targets[..] {
                [target] => match target {
                    ast::Expr::Name(name) => {
                        let symbol = Symbol::make(SymbolKind::Variable, name, Defn(assign));
                        errors.add(scope.insert(symbols, &name.id, symbol));
                    }
                    _ => errors.add(TypeError::new(
                        target.range(),
                        "only name targets are supported",
                    )),
                },

                _ => {
                    errors.add(TypeError::new(
                        assign.range,
                        "only single target assignments are support",
                    ));
                }
            },
            ast::Stmt::AnnAssign(assign) => match &*assign.target {
                ast::Expr::Name(name) => {
                    let mut symbol = Symbol::make(SymbolKind::Variable, name, Decl(assign));
                    if assign.value.is_some() {
                        symbol.defn = assign.node_index.load();
                    }
                    errors.add(scope.insert(symbols, &name.id, symbol));
                }
                _ => errors.add(TypeError::new(
                    assign.target.range(),
                    "only name targets are supported",
                )),
            },
            ast::Stmt::FunctionDef(func_def) => {
                let name = &func_def.name;
                let symbol = Symbol::make(SymbolKind::Function, name, Decl(func_def));
                errors.add(scope.insert(symbols, &name.id, symbol));
            }
            ast::Stmt::Nonlocal(nonlocal) => {
                for name in &nonlocal.names {
                    let symbol = Symbol::make(SymbolKind::Nonlocal, name, Decl(nonlocal));
                    errors.add(scope.insert(symbols, &name.id, symbol));
                }
            }
            _ => {}
        }
    }
    Outcome::mixed(scope, errors)
}
