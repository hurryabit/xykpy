use anyhow::bail;
use xykpy::resolver::Resolver;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        bail!("Usage: {} <file>", args[0]);
    }
    let file = &args[1];
    let source = std::fs::read_to_string(file)?;
    let parsed = parser::parse_module(&source)?;
    let module = xykpy::indexed::IndexedModule::new(parsed);

    let resolver = Resolver::new(module.syntax());
    let outcome = resolver.run();
    for error in outcome.errors {
        println!("ERROR @ {:?}: {}", error.range, error.message);
    }

    println!("{:#?}", outcome.value);

    // for (name, id) in scope.entries() {
    //     let symbol = symbols.get(*id);
    //     println!(
    //         "{kind:?}({name}) @ {range:?}= {symbol:?}",
    //         kind = symbol.kind,
    //         name = name,
    //         range = symbol.name_range,
    //         symbol = symbol,
    //     );
    // }

    Ok(())
}
