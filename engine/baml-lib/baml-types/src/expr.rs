// use moniker::{Binder, BoundTerm, Scope, Var};
use std::sync::Arc;

use crate::BamlValueWithMeta;

pub type Name = String;

/// A BAML expression term.
/// T is the type of the BamlValue metadata.
/// U is the type of arbitrary metadata, for example types.
// #[derive(Debug, Clone, BoundTerm)]
#[derive(Debug, Clone)]
pub enum Expr<T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug> {
    Atom(BamlValueWithMeta<U>,T),
    LLMFunction(Name, T),
    Var(Name, T),
    Lambda(Vec<Name>, Arc<Expr<T,U>>, T),
    App(Arc<Expr<T,U>>, Arc<Expr<T,U>>, T),
    Let(Name, Arc<Expr<T,U>>, Arc<Expr<T,U>>, T), // let name = expr in body
    ArgsTuple(Vec<Expr<T,U>>, T),
}

impl <T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug> Expr<T,U> {
    pub fn meta(&self) -> &T {
        match self {
            Expr::Atom(_, meta) => meta,
            Expr::LLMFunction(_, meta) => meta,
            Expr::Var(_, meta) => meta,
            Expr::Lambda(_, _, meta) => meta,
            Expr::App(_, _, meta) => meta,
            Expr::ArgsTuple(_, meta) => meta,
            Expr::Let(_, _, _, meta) => meta,
        }
    }

    pub fn into_meta(self) -> T {
        match self {
            Expr::Atom(_, meta) => meta,
            Expr::LLMFunction(_, meta) => meta,
            Expr::Var(_, meta) => meta,
            Expr::Lambda(_, _, meta) => meta,
            Expr::App(_, _, meta) => meta,
            Expr::ArgsTuple(_, meta) => meta,
            Expr::Let(_, _, _, meta) => meta,
        }
    }
}