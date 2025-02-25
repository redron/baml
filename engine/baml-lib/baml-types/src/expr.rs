#[macro_use]
extern crate moniker;

// use moniker::{Embed, Binder, Rec, Scope, Var};
use moniker::{Binder, Scope, Var};
use std::sync::Arc;

use crate::BamlValueWithMeta;

#[derive(Debug, Clone, BoundTerm)]
pub enum Expr<T,U> {
    Atom(BamlValueWithMeta<U>,T),
    Var(Var<String>),
    Lambda(Scope<Binder<String>, Arc<Expr<T,U>>>),
    App(Arc<Expr<T,U>>, Arc<Expr<T,U>>, T)
}
