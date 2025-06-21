use anyhow::bail;

fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    if args.len() != 2 {
        bail!("Usage: {} <file>", args[0]);
    }
    let file = &args[1];
    let source = std::fs::read_to_string(file)?;
    let parsed = parser::parse_module(&source)?;
    let module = xykpy::indexed::IndexedModule::new(parsed);

    let res = xykpy::table::collect_type_decls(&module.syntax().body);
    for error in res.errors {
        println!("ERROR @ {:?}: {}", error.range, error.message);
    }
    for (name, decl) in res.inner {
        println!("DECL @ {:?}: {} = {:?}", decl.name_range, name, decl);
    }

    Ok(())
}
