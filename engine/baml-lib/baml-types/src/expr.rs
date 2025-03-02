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
    LLMFunction(Name, Vec<Name>, T),
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
            Expr::LLMFunction(_, _, meta) => meta,
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
            Expr::LLMFunction(_, _, meta) => meta,
            Expr::Var(_, meta) => meta,
            Expr::Lambda(_, _, meta) => meta,
            Expr::App(_, _, meta) => meta,
            Expr::ArgsTuple(_, meta) => meta,
            Expr::Let(_, _, _, meta) => meta,
        }
    }
}

impl <T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug> Expr<T,U> {
    pub fn as_atom(&self) -> Option<&BamlValueWithMeta<U>> {
        match self {
            Expr::Atom(atom, _) => Some(atom),
            _ => None,
        }
    }

    pub fn dump_str(&self) -> String {
        match self {
            Expr::Atom(atom, _) => atom.clone().value().to_string(),
            Expr::LLMFunction(name, _, _) => name.clone(),
            Expr::Var(name, _) => name.clone(),
            Expr::Lambda(args, body, _) => format!("\\{:?} -> {}", args, body.dump_str()),
            Expr::App(func, args, _) => {
                let args_str = match args.as_ref() {
                    Expr::ArgsTuple(args, _) => args.iter().map(|arg| arg.dump_str()).collect::<Vec<_>>().join(", "),
                    _ => format!("(NON_ARGS_TUPLE {})", args.dump_str()),
                };
                format!("{}({})", func.dump_str(), args_str)
            },
            Expr::Let(name, expr, body, _) => format!("Let {} = {} in\n{}", name, expr.dump_str(), body.dump_str()),
            Expr::ArgsTuple(args, _) => format!("ArgsTuple({:?})", args.iter().map(|arg| arg.dump_str()).collect::<Vec<_>>()),
        }
    }
}
