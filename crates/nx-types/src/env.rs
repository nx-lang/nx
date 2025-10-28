//! Type environment for tracking type bindings.

use crate::Type;
use nx_hir::{ExprId, Name};
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// A binding of a name to a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeBinding {
    /// The name being bound
    pub name: Name,
    /// The type assigned to this name
    pub ty: Arc<Type>,
}

impl TypeBinding {
    /// Creates a new type binding.
    pub fn new(name: Name, ty: Type) -> Self {
        Self {
            name,
            ty: Arc::new(ty),
        }
    }
}

/// Type environment mapping names and expressions to types.
///
/// This tracks:
/// - Variable and function types by name
/// - Expression types by ExprId
#[derive(Debug, Clone, Default)]
pub struct TypeEnvironment {
    /// Name → Type mappings
    bindings: FxHashMap<Name, Arc<Type>>,
    /// ExprId → Type mappings
    expr_types: FxHashMap<ExprId, Arc<Type>>,
}

impl TypeEnvironment {
    /// Creates a new empty type environment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds a name to a type.
    pub fn bind(&mut self, name: Name, ty: Type) {
        self.bindings.insert(name, Arc::new(ty));
    }

    /// Removes a binding for a name, returning the previous type if it existed.
    pub fn remove(&mut self, name: &Name) -> Option<Arc<Type>> {
        self.bindings.remove(name)
    }

    /// Looks up the type of a name.
    pub fn lookup(&self, name: &Name) -> Option<&Type> {
        self.bindings.get(name).map(|arc| arc.as_ref())
    }

    /// Records the type of an expression.
    pub fn set_expr_type(&mut self, expr: ExprId, ty: Type) {
        self.expr_types.insert(expr, Arc::new(ty));
    }

    /// Gets the type of an expression.
    pub fn get_expr_type(&self, expr: ExprId) -> Option<&Type> {
        self.expr_types.get(&expr).map(|arc| arc.as_ref())
    }

    /// Returns all name bindings.
    pub fn bindings(&self) -> impl Iterator<Item = (&Name, &Type)> {
        self.bindings.iter().map(|(name, ty)| (name, ty.as_ref()))
    }

    /// Returns the number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Returns true if there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Clears all bindings.
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.expr_types.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use la_arena::Arena;
    use nx_hir::ast::Expr;

    #[test]
    fn test_env_creation() {
        let env = TypeEnvironment::new();
        assert!(env.is_empty());
    }

    #[test]
    fn test_env_bind() {
        let mut env = TypeEnvironment::new();
        let name = Name::new("x");
        env.bind(name.clone(), Type::int());

        assert_eq!(env.len(), 1);
        assert_eq!(env.lookup(&name), Some(&Type::int()));
    }

    #[test]
    fn test_env_lookup_missing() {
        let env = TypeEnvironment::new();
        let name = Name::new("undefined");
        assert_eq!(env.lookup(&name), None);
    }

    #[test]
    fn test_env_rebind() {
        let mut env = TypeEnvironment::new();
        let name = Name::new("x");

        env.bind(name.clone(), Type::int());
        assert_eq!(env.lookup(&name), Some(&Type::int()));

        env.bind(name.clone(), Type::string());
        assert_eq!(env.lookup(&name), Some(&Type::string()));
    }

    #[test]
    fn test_env_remove() {
        let mut env = TypeEnvironment::new();
        let name = Name::new("temp");

        env.bind(name.clone(), Type::bool());
        let removed = env.remove(&name);
        assert!(removed.is_some());
        assert!(env.lookup(&name).is_none());
    }

    #[test]
    fn test_env_expr_types() {
        let mut env = TypeEnvironment::new();
        let mut arena = Arena::new();
        let expr_id = arena.alloc(Expr::Literal(nx_hir::ast::Literal::Int(42)));

        env.set_expr_type(expr_id, Type::int());
        assert_eq!(env.get_expr_type(expr_id), Some(&Type::int()));
    }

    #[test]
    fn test_env_clear() {
        let mut env = TypeEnvironment::new();
        env.bind(Name::new("x"), Type::int());
        assert!(!env.is_empty());

        env.clear();
        assert!(env.is_empty());
    }

    #[test]
    fn test_type_binding() {
        let name = Name::new("test");
        let binding = TypeBinding::new(name.clone(), Type::string());

        assert_eq!(binding.name, name);
        assert_eq!(*binding.ty, Type::string());
    }
}
