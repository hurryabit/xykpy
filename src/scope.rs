#![allow(dead_code)]
use std::collections::HashSet;

use crate::symbol::SymbolId;

pub use id::ScopeId;
pub use table::ScopeTable;

#[derive(Debug)]
pub struct Scope {
    node: ast::NodeIndex,
    parent: Option<ScopeId>,
    children: Vec<ScopeId>,
    symbols: HashSet<SymbolId>,
}

mod id {
    use std::num::NonZeroU16;

    #[derive(Clone, Copy, Debug)]
    pub struct ScopeId(NonZeroU16);

    impl ScopeId {
        pub(super) fn from_index(index: usize) -> Self {
            let short: u16 = index.try_into().expect("More than 16k scopes? Wow!");
            Self((short + 1).try_into().expect("More than 16k scopes? Wow!"))
        }

        pub(super) fn into_index(self) -> usize {
            let short: u16 = self.0.into();
            short as usize - 1
        }
    }
}

mod table {
    use super::*;

    #[derive(Debug)]
    pub struct ScopeTable {
        root_id: ScopeId,
        scopes: Vec<Scope>,
    }

    impl ScopeTable {
        pub fn new(root_node: ast::NodeIndex) -> Self {
            let root_id = ScopeId::from_index(0);
            let root = Scope {
                node: root_node,
                parent: None,
                children: Vec::new(),
                symbols: HashSet::new(),
            };
            let scopes = Vec::from([root]);
            Self { root_id, scopes }
        }

        pub fn root_id(&self) -> ScopeId {
            self.root_id
        }

        pub fn root(&self) -> &Scope {
            &self.scopes[0]
        }

        pub fn make_scope(&mut self, node: ast::NodeIndex, parent: ScopeId) -> ScopeId {
            let index = self.scopes.len();
            let scope = Scope {
                node,
                parent: Some(parent),
                children: Vec::new(),
                symbols: HashSet::new(),
            };
            self.scopes.push(scope);
            ScopeId::from_index(index)
        }

        pub fn get(&self, id: ScopeId) -> &Scope {
            &self.scopes[id.into_index()]
        }

        pub fn add_symbol(&mut self, scope: ScopeId, symbol: SymbolId) -> bool {
            self.scopes[scope.into_index()].symbols.insert(symbol)
        }
    }
}
