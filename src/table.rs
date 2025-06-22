#![allow(dead_code)]
use std::collections::HashMap;

use text_size::Ranged;

use crate::error::{Errors, ErrorsBuilder, Outcome, TypeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Class,
    Alias,
    Variable,
    Function,
}

impl SymbolKind {
    fn is_type(self) -> bool {
        use SymbolKind::*;
        match self {
            Alias | Class => true,
            Function | Variable => false,
        }
    }
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            SymbolKind::Class => "class",
            SymbolKind::Alias => "type alias",
            SymbolKind::Variable => "variable",
            SymbolKind::Function => "function",
        };
        f.write_str(text)
    }
}

fn no_node_index() -> ast::NodeIndex {
    ast::AtomicNodeIndex::dummy().load()
}

#[derive(Debug)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub name: ast::NodeIndex,
    pub name_range: text_size::TextRange,
    pub decl: ast::NodeIndex,
    pub defn: ast::NodeIndex,
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
        if self.kind == SymbolKind::Variable && later.kind == SymbolKind::Variable {
            let conflict = self.is_decl() && later.is_decl();
            let decl = if self.is_decl() || !later.is_decl() {
                self
            } else {
                later
            };
            let defn = ast::NodeIndex::min(self.defn, later.defn);
            let merged = Symbol { defn, ..*decl };
            (Some(merged), conflict)
        } else {
            (None, true)
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
    let mut scope = ScopeTable::new();
    let mut errors = ErrorsBuilder::new();
    for stmt in stmts {
        match stmt {
            ast::Stmt::ClassDef(class_def) => {
                let name = &class_def.name;
                let symbol = Symbol {
                    kind: SymbolKind::Class,
                    name: name.node_index.load(),
                    name_range: name.range,
                    decl: class_def.node_index.load(),
                    defn: class_def.node_index.load(),
                };
                errors.add(scope.insert(symbols, &name.id, symbol));
            }
            ast::Stmt::TypeAlias(alias_def) => match &*alias_def.name {
                ast::Expr::Name(name) => {
                    let symbol = Symbol {
                        kind: SymbolKind::Alias,
                        name: name.node_index.load(),
                        name_range: name.range,
                        decl: alias_def.node_index.load(),
                        defn: alias_def.node_index.load(),
                    };
                    errors.add(scope.insert(symbols, &name.id, symbol));
                }
                _ => {
                    unreachable!("The grammar only allows `type A = ...` and `type A[...] = ...`.");
                }
            },
            ast::Stmt::Assign(assign) => match &assign.targets[..] {
                [target] => match target {
                    ast::Expr::Name(name) => {
                        let symbol = Symbol {
                            kind: SymbolKind::Variable,
                            name: name.node_index.load(),
                            name_range: name.range,
                            decl: no_node_index(),
                            defn: assign.node_index.load(),
                        };
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
                    let symbol = Symbol {
                        kind: SymbolKind::Variable,
                        name: name.node_index.load(),
                        name_range: name.range,
                        decl: assign.node_index.load(),
                        defn: if assign.value.is_some() {
                            assign.node_index.load()
                        } else {
                            no_node_index()
                        },
                    };
                    errors.add(scope.insert(symbols, &name.id, symbol));
                }
                _ => errors.add(TypeError::new(
                    assign.target.range(),
                    "only name targets are supported",
                )),
            },
            ast::Stmt::FunctionDef(func_def) => {
                let name = &func_def.name;
                let symbol = Symbol {
                    kind: SymbolKind::Function,
                    name: name.node_index.load(),
                    name_range: name.range,
                    decl: func_def.node_index.load(),
                    defn: func_def.node_index.load(),
                };
                errors.add(scope.insert(symbols, &name.id, symbol));
            }
            _ => {}
        }
    }
    Outcome::mixed(scope, errors)
}
