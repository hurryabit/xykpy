#![allow(dead_code)]

use std::collections::HashMap;

use lookup::{ScopeLookup, ScopeLoopkupBuilder};

use crate::{
    error::{ErrorsBuilder, Outcome},
    scope::ScopeTable,
    symbol::{SymbolId, SymbolTable},
};

mod lookup;

#[derive(Debug)]
pub struct Resolution {
    symbols: SymbolTable,
    scopes: ScopeTable,
    nodes: HashMap<ast::NodeIndex, SymbolId>,
}

pub struct Resolver<'m> {
    module: &'m ast::ModModule,
    resolution: Resolution,
    errors: ErrorsBuilder,
    env: Vec<ScopeLookup<'m>>,
}

impl<'m> Resolver<'m> {
    pub fn new(module: &'m ast::ModModule) -> Self {
        let symbols = SymbolTable::new();
        let scopes = ScopeTable::new(module.node_index.load());
        let nodes = HashMap::new();
        let resolution = Resolution { symbols, scopes, nodes };
        let errors = ErrorsBuilder::new();
        let env = Vec::new();
        Self { module, resolution, errors, env }
    }

    pub fn run(mut self) -> Outcome<Resolution> {
        let root_id = self.resolution.scopes.root_id();
        let mut builder = ScopeLoopkupBuilder::new(
            &mut self.resolution.symbols,
            &mut self.resolution.scopes,
            &mut self.errors,
            root_id,
        );
        builder.add_block(&self.module.body);
        Outcome::mixed(self.resolution, self.errors)
    }
}
