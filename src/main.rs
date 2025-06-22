use anyhow::bail;
use xykpy::{
    error::Outcome,
    table::{SymbolTable, block_scope},
};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        bail!("Usage: {} <file>", args[0]);
    }
    let file = &args[1];
    let source = std::fs::read_to_string(file)?;
    let parsed = parser::parse_module(&source)?;
    let module = xykpy::indexed::IndexedModule::new(parsed);

    let mut symbols = SymbolTable::new();
    let Outcome {
        value: scope,
        errors,
    } = block_scope(&mut symbols, &module.syntax().body);
    for error in errors {
        println!("ERROR @ {:?}: {}", error.range, error.message);
    }
    for (name, id) in scope.entries() {
        let symbol = symbols.get(*id);
        println!(
            "{kind:?}({name}) @ {range:?}= {symbol:?}",
            kind = symbol.kind,
            name = name,
            range = symbol.name_range,
            symbol = symbol,
        );
    }

    Ok(())
}
