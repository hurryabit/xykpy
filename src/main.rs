use anyhow::bail;
use ruff_python_ast::{self as ast, identifier::Identifier, statement_visitor::StatementVisitor};
use ruff_python_parser as parser;

use ast::HasNodeIndex;

mod indexed;

struct StmtPrinter<'a> {
    source: &'a str,
}

impl<'a> StmtPrinter<'a> {
    fn new(source: &'a str) -> Self {
        Self { source }
    }
}

impl<'a> ast::statement_visitor::StatementVisitor<'a> for StmtPrinter<'a> {
    fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
        if !ast::helpers::is_compound_statement(stmt) {
            let index = stmt.node_index().load();
            let ident = &self.source[stmt.identifier()];
            println!("{:?} -> {}", index, ident);
        }
        ast::statement_visitor::walk_stmt(self, stmt);
    }
}

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        bail!("Usage: {} <file>", args[0]);
    }
    let file = &args[1];
    let source = std::fs::read_to_string(file)?;
    let parsed = parser::parse_module(&source)?;
    let module = indexed::IndexedModule::new(parsed);

    let mut printer = StmtPrinter::new(&source);
    printer.visit_body(&module.syntax().body);

    Ok(())
}
