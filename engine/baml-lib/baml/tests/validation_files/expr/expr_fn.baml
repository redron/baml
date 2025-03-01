fn foo(x: int) -> int {
    x
}

fn bar(x: int, y: string) -> int {
    let y = go(x);
    let z = go2(x,y);
    go3(z)
}