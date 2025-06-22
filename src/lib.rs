pub mod error;
pub mod indexed;
pub mod resolver;
pub mod scope;
pub mod symbol;

trait HasId {
    fn id(&self) -> &ast::name::Name;
}

impl HasId for ast::Identifier {
    fn id(&self) -> &ast::name::Name {
        &self.id
    }
}

impl HasId for ast::ExprName {
    fn id(&self) -> &ast::name::Name {
        &self.id
    }
}
