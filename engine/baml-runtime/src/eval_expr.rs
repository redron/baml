use std::collections::HashMap;
use std::sync::Arc;

use crate::BamlRuntime;
use baml_types::expr::{Expr, Name};
use baml_types::BamlValueWithMeta;

pub struct EvalEnv<T, U> {
    context: HashMap<Name, Arc<Expr<T, U>>>,
    runtime: Arc<BamlRuntime>,
}

pub async fn step<T: Clone, U: Clone>(
    ctx: HashMap<Name, Expr<T, U>>,
    expr: &Expr<T, U>,
) -> anyhow::Result<Expr<T, U>> {
    match expr {
        Expr::Var(var_name, _) => {
            let val = ctx
                .get(var_name)
                .ok_or(anyhow::anyhow!("Variable not found: {}", var_name))?;
            Ok(val.clone())
        }
        Expr::Atom(_, _) => Ok(expr.clone()),
        Expr::Lambda(_, _, _) => Ok(expr.clone()),
        Expr::App(f, x, _) => step_app(ctx, f, x).await,
        Expr::LLMFunction(fn_name, _) => {
            Err(anyhow::anyhow!("Reached bare LLM function: {:?}", fn_name))
        }
    }
}

async fn step_app<T: Clone, U: Clone>(
    ctx: HashMap<Name, Expr<T, U>>,
    f: &Expr<T, U>,
    x: &Expr<T, U>,
) -> anyhow::Result<Expr<T, U>> {
    match f.as_ref() {
        Expr::Lambda(_, body, _) => {
            let mut new_ctx = ctx.clone();
            new_ctx.insert(f.as_ref().borrow().name.clone(), x.clone());
            step(new_ctx, body)
        }
        _ => Err(anyhow::anyhow!("Not a function: {}", f)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BamlRuntime;

    // Make a testing runtime. It assumes the presence of
    // OPENAI_API_KEY environment variable.
    fn runtime() -> BamlRuntime {
        let ir = make_test_ir(r##"
        function count_words(text: string) -> int {
          client GPT35
          prompt #"
          "#
        }
        "##).unwrap();
    }   
    
}
