use std::collections::HashMap;

use crate::error::{ErrorsBuilder, TypeError, WithErrors, WithErrorsExt};

#[derive(Debug)]
pub enum TypeDeclKind {
    // TypeVar,
    Class,
    Alias,
}

#[derive(Debug)]
pub struct TypeDecl {
    pub kind: TypeDeclKind,
    pub decl: ast::NodeIndex,
    pub name: ast::NodeIndex,
    pub name_range: text_size::TextRange,
}

pub fn collect_type_decls(
    stmts: &Vec<ast::Stmt>,
) -> WithErrors<HashMap<&ast::name::Name, TypeDecl>> {
    use std::collections::hash_map::Entry;
    let mut res = HashMap::<&ast::name::Name, TypeDecl>::new();
    let mut errors = ErrorsBuilder::new();
    for stmt in stmts {
        let (name, decl) = match stmt {
            ast::Stmt::ClassDef(class_def) => {
                let decl = TypeDecl {
                    kind: TypeDeclKind::Class,
                    decl: class_def.node_index.load(),
                    name: class_def.name.node_index.load(),
                    name_range: class_def.name.range,
                };
                let name = &class_def.name.id;
                (name, decl)
            }
            ast::Stmt::TypeAlias(alias_def) => match &*alias_def.name {
                ast::Expr::Name(name) => {
                    let decl = TypeDecl {
                        kind: TypeDeclKind::Alias,
                        decl: alias_def.node_index.load(),
                        name: name.node_index.load(),
                        name_range: name.range,
                    };
                    let name = &name.id;
                    (name, decl)
                }
                _ => {
                    unreachable!("The grammar only allows `type A = ...` and `type A[...] = ...`.");
                }
            },
            _ => continue,
        };
        match res.entry(name) {
            Entry::Occupied(occupied_entry) => errors.add(TypeError::new(
                decl.name_range,
                format!(
                    "duplicate type definition, first definition at {:?}",
                    occupied_entry.get().name_range,
                ),
            )),
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(decl);
            }
        }
    }
    res.with_errors(errors)
}
