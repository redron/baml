use moniker::{Binder, BoundTerm, Scope, Var};
use std::sync::Arc;

use crate::BamlValueWithMeta;

type Name = String;

/// A BAML expression term.
/// T is the type of the BamlValue metadata.
/// U is the type of arbitrary metadata, for example types.
#[derive(Debug, Clone, BoundTerm)]
pub enum Expr<T,U> {
    Atom(BamlValueWithMeta<U>,T),
    Var(Var<Name>, T),
    Lambda(Scope<Vec<Binder<Name>>, Arc<Expr<T,U>>>, T),
    App(Arc<Expr<T,U>>, Arc<Expr<T,U>>, T)
}



pub fn step<T,U>(
    context: &Scope<Binder<Vec<Name>>, Arc<Expr<T,U>>>,
    expr: &mut Expr<T,U>
) -> anyhow::Result<Expr<T,U>>
where
    T: Clone + std::fmt::Debug,
    U: Clone + std::fmt::Debug,
{
    match expr {
        Expr::Var(v, t) => {
            if let Some(e) = context.get(v) {
                Ok(e.clone())
            } else {
                Err(anyhow::anyhow!("Variable not found: {}", v))
            }
        }
        Expr::App(f, x, t) => {
            Some(Expr::App(f.clone(), x.clone(), t.clone()))
        }
        _ => None
    }
}
