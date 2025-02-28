// use moniker::{Binder, BoundTerm, Scope, Var};
use std::sync::Arc;

use crate::BamlValueWithMeta;

pub type Name = String;

/// A BAML expression term.
/// T is the type of the BamlValue metadata.
/// U is the type of arbitrary metadata, for example types.
// #[derive(Debug, Clone, BoundTerm)]
#[derive(Debug, Clone)]
pub enum Expr<T,U> {
    Atom(BamlValueWithMeta<U>,T),
    LLMFunction(Name, T),
    Var(Name, T),
    Lambda(Vec<Name>, Arc<Expr<T,U>>, T),
    App(Arc<Expr<T,U>>, Arc<Expr<T,U>>, T)
}
