use ast::HasNodeIndex;
use text_size::Ranged;

pub use id::SymbolId;
pub use table::SymbolTable;

use crate::scope::ScopeId;

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
    pub scope: ScopeId,
    pub name: ast::NodeIndex,
    pub name_range: text_size::TextRange,
    pub decl: ast::NodeIndex,
    pub defn: ast::NodeIndex,
}

static_assertions::const_assert_eq!(std::mem::size_of::<Symbol>(), 24);

pub(crate) enum DeclOrDefn<T> {
    Decl(T),
    Defn(T),
    DeclAndDefn(T),
}

impl Symbol {
    fn is_decl(&self) -> bool {
        self.decl != no_node_index()
    }

    // Returns whether the two symbols conflict.
    pub(crate) fn merge(&self, later: &Symbol) -> (Option<Symbol>, bool) {
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

    pub(crate) fn make(
        kind: SymbolKind,
        scope: ScopeId,
        name: impl HasNodeIndex + Ranged,
        decl_defn: DeclOrDefn<impl HasNodeIndex>,
    ) -> Self {
        let (decl, defn) = match decl_defn {
            DeclOrDefn::Decl(node) => (node.node_index().load(), no_node_index()),
            DeclOrDefn::Defn(node) => (no_node_index(), node.node_index().load()),
            DeclOrDefn::DeclAndDefn(node) => {
                let node_index = node.node_index().load();
                (node_index, node_index)
            }
        };
        Self {
            kind,
            scope,
            name: name.node_index().load(),
            name_range: name.range(),
            decl,
            defn,
        }
    }
}

mod id {
    #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
    pub struct SymbolId(pub(super) u32);
}

mod table {
    use super::*;

    #[derive(Debug)]
    pub struct SymbolTable(Vec<super::Symbol>);

    impl SymbolTable {
        pub fn new() -> Self {
            Self(Vec::new())
        }

        pub fn insert(&mut self, symbol: Symbol) -> SymbolId {
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
}
