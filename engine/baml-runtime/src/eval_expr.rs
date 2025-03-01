use std::collections::HashMap;
use std::sync::Arc;

use crate::BamlRuntime;
use baml_types::expr::{Expr, Name};
use baml_types::BamlValueWithMeta;

pub struct EvalEnv<T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug> {
    context: HashMap<Name, Arc<Expr<T, U>>>,
    runtime: Arc<BamlRuntime>,
}

pub async fn step<T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug>(
    ctx: HashMap<Name, Expr<T, U>>,
    expr: &Expr<T, U>,
) -> anyhow::Result<Expr<T, U>> {
    Box::pin(async move {
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
            Expr::ArgsTuple(args, meta) => {
                let mut resolved_args = Vec::new();
                for arg in args {
                    let val = to_value(&ctx, arg).await?;
                    resolved_args.push(Expr::Atom(val.unwrap(), arg.meta().clone()));
                }
                Ok(Expr::ArgsTuple(resolved_args, meta.clone()))
            }
        }
    })
    .await
}

async fn step_app<T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug>(
    ctx: HashMap<Name, Expr<T, U>>,
    f: &Expr<T, U>,
    x: &Expr<T, U>,
) -> anyhow::Result<Expr<T, U>> {
    match (f, x) {
        (Expr::Lambda(params, body, _), Expr::ArgsTuple(args, _)) => {
            // Apply a lambda to its argument.
            // (\(x,y) (x + y))(1,2)
            if params.len() != args.len() {
                return Err(anyhow::anyhow!(
                    "Wrong number of arguments: {} != {}",
                    params.len(),
                    args.len()
                ));
            }
            let mut new_ctx = ctx.clone();
            for (param, arg) in params.iter().zip(args.iter()) {
                new_ctx.insert(param.clone(), arg.clone());
            }
            subst(&new_ctx, body)
        }
        _ => Err(anyhow::anyhow!("Not a function: {:?}", f)),
    }
}

/// Replace variables in an expression with their expressions.
/// This implementation completely ignores captures. TODO: fix.
fn subst<T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug>(
    ctx: &HashMap<Name, Expr<T, U>>,
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
        Expr::Lambda(_, body, _) => subst(ctx, body),
        Expr::App(f, x, meta) => {
            let f = subst(ctx, f)?;
            let x = subst(ctx, x)?;
            Ok(Expr::App(Arc::new(f), Arc::new(x), meta.clone()))
        }
        Expr::ArgsTuple(args, meta) => {
            let args = args
                .iter()
                .map(|arg| subst(&ctx, arg))
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok(Expr::ArgsTuple(args, meta.clone()))
        }
        Expr::LLMFunction(_, _) => Ok(expr.clone()),
    }
}

/// Fully evaluate an expression to a value.
async fn to_value<T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug>(
    ctx: &HashMap<Name, Expr<T, U>>,
    expr: &Expr<T, U>,
) -> anyhow::Result<Option<BamlValueWithMeta<U>>> {
    match expr {
        Expr::Atom(value, _) => Ok(Some(value.clone())),
        other => {
            let new_expr = step(ctx.clone(), other).await?;
            to_value(ctx, &new_expr).await
        }
    }
}

#[cfg(test)]
mod tests {
    use baml_types::{BamlMap, BamlValue};
    use internal_baml_core::ir::repr::make_test_ir;

    use super::*;
    use crate::BamlRuntime;

    // Make a testing runtime. It assumes the presence of
    // OPENAI_API_KEY environment variable.
    fn runtime() -> BamlRuntime {
        let content = r##"
        client<llm> GPT35 {
          provider baml-openai-chat
          options {
            model gpt-3.5-turbo
            api_key env.OPENAI_API_KEY
          }
        }

        function CountWords(text: string) -> int {
          client GPT35
          prompt #"
          "#
        }

        fn second(x: string, y: string) -> int {
          let x1 = LlmParseInt(x);
          let y1 = LlmParseInt(y);
          y1
        }

        function LlmParseInt(inp: string) -> int {
          client GPT35
          prompt #"
            Parse the following text as an integer:
            {{ inp }}

            {{ ctx.output_format}}
          "#
        }

        test TestSecond {
          functions [second]
          args {
            x: "123"
            y: "456"
          }
        }
        "##;
        BamlRuntime::from_file_content(
            ".",
            &HashMap::from([("main.baml", content)]),
            HashMap::from([("OPENAI_API_KEY", env!("OPENAI_API_KEY"))]),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_eval_expr() {
        let rt = runtime();
        let ctx = rt.create_ctx_manager(BamlValue::String("test".to_string()), None);
        let params = BamlMap::from([(
            "inp".to_string(),
            BamlValue::String("123".to_string().to_string()),
        )]);
        let res = rt
            .call_function("LlmParseInt".to_string(), &params, &ctx, None, None)
            .await
            .0
            .unwrap();
        let res2 = res
            .parsed()
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap();
        dbg!(res2);
        assert!(false);
    }
}
