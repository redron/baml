fn Foo(x: int) -> int {
    x
    y
}

fn Bar(x: int, y: string) -> int {
    let y = go(x);
    let z = go2(x,y);
    go3(z)
}

test TestBar {
    functions [Bar]
    args {
        x 1
        y "hello"
    }
}

// error: Invalid function body
//   -->  expr/invalid_expr_fn.baml:1
//    | 
//    | 
//  1 | fn Foo(x: int) -> int {
//  2 |     x
//    | 
// error: Error validating: This line is invalid. It does not start with any known Baml schema keyword.
//   -->  expr/invalid_expr_fn.baml:2
//    | 
//  1 | fn Foo(x: int) -> int {
//  2 |     x
//  3 |     y
//    | 
