use std::collections::HashMap;
use std::sync::Arc;

use crate::{BamlRuntime, FunctionResult};
use baml_types::expr::{Expr, Name};
use baml_types::{BamlMap, BamlValue, BamlValueWithMeta};
use internal_baml_core::ir::repr::IntermediateRepr;
pub struct EvalEnv<'a, T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug + Default> {
    pub context: HashMap<Name, Expr<T, U>>,
    pub runtime: &'a BamlRuntime,
}

impl <'a, T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug + Default> EvalEnv<'a, T, U> {
    pub fn dump_ctx(&self) -> String {
        self.context.iter().map(|(k, v)| format!("{}: {}", k, v.dump_str())).collect::<Vec<_>>().join("\n")
    }
}

/// Perform a single execution step. It only performs beta reduction.
pub async fn step<'a, T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug + Default>(
    env: &EvalEnv<'a, T, U>,
    expr: &Expr<T, U>,
) -> anyhow::Result<Expr<T, U>> {
    eprintln!("--------\nSTEP:\n{}\n\nCONTEXT\n {}\n\n", expr.dump_str(), env.dump_ctx());
    Box::pin(async move {
        match expr {
            Expr::Var(var_name, _) => {
                let val = env
                    .context
                    .get(var_name)
                    .ok_or(anyhow::anyhow!("Variable not found: {}", var_name))?;
                Ok(val.clone())
            },
            Expr::Let(name, val, body, meta) => {

                let mut new_context = env.context.clone();
                new_context.insert(name.clone(), (val.as_ref()).clone());

                let new_env = EvalEnv {
                    context: new_context,
                    runtime: env.runtime,
                };

                let new_body = subst(&new_env, body).await?;
                let new_body = eval_to_value(&new_env, &new_body).await?.expect("Couldn't go all the way");
                let new_body = Expr::Atom(new_body, meta.clone());

                // Ok(new_body)
                step(&new_env, &new_body).await
                // step(env, new_body).await?.clone()
            },
            Expr::Atom(_, _) => Ok(expr.clone()),
            Expr::Lambda(_, _, _) => Ok(expr.clone()),
            Expr::App(f, x, meta) => step_app(env, &subst(env, f).await?, x, meta).await,
            Expr::LLMFunction(fn_name, _, _) => {
                Err(anyhow::anyhow!("Reached bare LLM function: {:?}", fn_name))
            }
            Expr::ArgsTuple(args, meta) => {
                // let mut resolved_args = Vec::new();
                // for arg in args {
                //     let val = to_value(env, arg).await?;
                //     resolved_args.push(Expr::Atom(val.unwrap(), arg.meta().clone()));
                // }
                // Ok(Expr::ArgsTuple(resolved_args, meta.clone()))
                Err(anyhow::anyhow!("Reached bare ArgsTuple"))
            }
            _ => todo!(),
        }
    })
    .await
}

async fn step_app<'a,  T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug + Default>(
    env: &EvalEnv<'a, T, U>,
    f: &Expr<T, U>,
    x: &Expr<T, U>,
    meta: &T,
) -> anyhow::Result<Expr<T, U>> {
    eprintln!("STEP_APP:\nf:{}\nx:{}\nCTX:{}\n", f.dump_str(), x.dump_str(), env.dump_ctx());
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
            let mut new_ctx = env.context.clone();
            for (param, arg) in params.iter().zip(args.iter()) {
                new_ctx.insert(param.clone(), arg.clone());
            }
            let new_env = EvalEnv {
                context: new_ctx,
                runtime: env.runtime,
            };
            subst(&new_env, body).await
        }
        (Expr::LLMFunction(fn_name, arg_names, _), Expr::ArgsTuple(args, _)) => {
            let args: Vec<BamlValue> = args.clone().into_iter().map(|arg| arg.as_atom().unwrap().clone().value()).collect();
            let params = args.into_iter().zip(arg_names.iter()).map(|(arg, name)| (name.clone(), arg)).collect::<HashMap<_, _>>();
            let args_map = BamlMap::from_iter(params.into_iter());
            let ctx = env
                .runtime
                .create_ctx_manager(BamlValue::String("none".to_string()), None);
            let res: anyhow::Result<FunctionResult> = env
                .runtime
                .call_function(fn_name.clone(), &args_map, &ctx, None, None)
                .await
                .0;
            let val = res
                .unwrap()
                .parsed()
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .clone()
                .0
                .map_meta(|_| U::default());
            Ok(Expr::Atom(val, meta.clone()))
        }
        (Expr::Lambda(_,_,_), _) => {
            let new_f = subst(env, f).await?;
            Box::pin(step_app(env, &new_f, x, meta)).await
        }
        _ => Err(anyhow::anyhow!("Not a function: {:?}", f)),
    }
}

/// Replace variables in an expression with their expressions
/// from the environment context.
/// This implementation completely ignores captures. TODO: fix.
async fn subst<'a, T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug + Default>(
    env: &EvalEnv<'a, T, U>,
    expr: &Expr<T, U>,
) -> anyhow::Result<Expr<T, U>> {
    eprintln!("SUBST:\n{}\nCTX:{}\n", expr.dump_str(), env.dump_ctx());
    match expr {
        Expr::Var(var_name, _) => {
            let val = env.context
                .get(var_name)
                .ok_or(anyhow::anyhow!("Variable not found: {}", var_name))?;
            Ok(val.clone())
        }
        Expr::Atom(_, _) => Ok(expr.clone()),
        Expr::Lambda(_, body, _) => {
            // Box::pin the recursive call
            Box::pin(subst(env, body)).await
        },
        Expr::App(f, x, meta) => {
            // Box::pin both recursive calls
            let f = Box::pin(subst(env, f)).await?;
            let x = Box::pin(subst(env, x)).await?;
            Ok(Expr::App(Arc::new(f), Arc::new(x), meta.clone()))
        }
        Expr::ArgsTuple(args, meta) => {
            let mut new_args = Vec::new();
            for arg in args {
                // Box::pin the recursive call in the loop
                new_args.push(Box::pin(subst(env, arg)).await?);
            }
            Ok(Expr::ArgsTuple(new_args, meta.clone()))
        }
        Expr::LLMFunction(_, _, _) => Ok(expr.clone()),
        Expr::Let(name, value, body, meta) => {
            let mut new_ctx = env.context.clone();
            // Box::pin the eval_to_value call
            let new_value = Box::pin(subst(env, value)).await?;
            new_ctx.insert(name.clone(), new_value.clone());
            let new_env = EvalEnv {
                context: new_ctx,
                runtime: env.runtime,
            };
            // Box::pin the recursive subst call
            let new_body = Box::pin(subst(&new_env, body)).await?;
            Ok(Expr::Let(
                name.clone(),
                Arc::new(new_value),
                Arc::new(new_body),
                meta.clone(),
            ))
        }
    }
}

/// Fully evaluate an expression to a value.
pub async fn eval_to_value<'a, T: Clone + std::fmt::Debug, U: Clone + std::fmt::Debug + Default>(
    env: &EvalEnv<'a, T, U>,
    expr: &Expr<T, U>,
) -> anyhow::Result<Option<BamlValueWithMeta<U>>> {
    eprintln!("called to_value: {:?}", expr);
    let max_steps = 1000;
    let mut current_expr = expr.clone();

    for steps in 0..max_steps {
        match current_expr {
            Expr::Atom(value, _) => return Ok(Some(value.clone())),
            other => {
                let new_expr = step(env, &other).await?;

                if new_expr.temporary_same_state(expr) {
                    return Err(anyhow::anyhow!("Failed to make progress."));
                }
                current_expr = new_expr;
            }
        }
    }
    Err(anyhow::anyhow!("Max steps reached."))
}

/// Create a context from the expr_functions, top_level_assignments, and
/// functions in the IR.
pub fn initial_context(ir: &IntermediateRepr) -> HashMap<Name, Expr<(), ()>> {
    let mut ctx = HashMap::new();

    for expr_fn in ir.expr_fns.iter() {
        ctx.insert(expr_fn.elem.0.clone(), expr_fn.elem.1.clone());
    }
    for top_level_assignment in ir.toplevel_assignments.iter() {
        ctx.insert(
            top_level_assignment.elem.name.elem.clone(),
            top_level_assignment.elem.expr.elem.clone(),
        );
    }
    for llm_function in ir.functions.iter() {
        let params = llm_function.elem.inputs.iter().map(|arg| arg.0.clone()).collect::<Vec<_>>();
        ctx.insert(
            llm_function.elem.name.clone(),
            Expr::LLMFunction(llm_function.elem.name.clone(), params, ()),
        );
    }
    ctx
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

        fn Second(x: string, y: string) -> int {
          let x1 = LlmParseInt(x);
          let y1 = LlmParseInt(y);
          y1
        }

        fn DoId(x: int, y: int) -> int {
          let z = Double(x);
          z
        }

        function Double(my_inp: int) -> int {
          client GPT35
          prompt #"
          Double the following integer:
          {{ my_inp }}

          {{ ctx.output_format}}
          "#
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

        // let params = BamlMap::from([(
        //     "inp".to_string(),
        //     BamlValue::String("123".to_string().to_string()),
        // )]);
        // let res = rt
        //     .call_function("LlmParseInt".to_string(), &params, &ctx, None, None)
        //     .await
        //     .0
        //     .unwrap();
        // let res2 = res.parsed().as_ref().unwrap().as_ref().unwrap();
        // dbg!(res2);

        // dbg!(&rt.inner.ir.expr_fns);
        // let fns: Vec<_> = rt
        //     .inner
        //     .ir
        //     .walk_expr_fns()
        //     .into_iter()
        //     .map(|w| (w.item.elem.0.clone(), w.item.elem.1.dump_str()))
        //     .collect::<Vec<_>>();
        // fns.iter()
        //     .for_each(|(name, fn_str)| eprintln!("{}: {}", name, fn_str));

        // let params = BamlMap::from([(
        //     "x".to_string(),
        //     BamlValue::String("123".to_string().to_string()),
        // ), (
        //     "y".to_string(),
        //     BamlValue::String("456".to_string().to_string()),
        // )]);
        // let res3 = rt
        //     .call_function("Second".to_string(), &params, &ctx, None, None)
        //     .await
        //     .0.unwrap();
        // let res4 = res3.parsed().as_ref().unwrap().as_ref().unwrap();
        // dbg!(res4);

        let params = BamlMap::from([(
            "x".to_string(),
            BamlValue::Int(888),
        ), ("y".to_string(), BamlValue::Int(999))]);
        let res3 = rt
            .call_function("DoId".to_string(), &params, &ctx, None, None)
            .await
            .0.unwrap();
        let res4 = res3.parsed().as_ref().unwrap().as_ref().unwrap();
        dbg!(res4);

        assert!(false);
    }
}
