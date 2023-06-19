use anyhow::Result;

fn main() -> Result<()> {
    let program = r#"
/* x'+y=w -> x+y=z, w=z' */
S(#x) :-: A(#y, #w) => #x = A(#y, #z), #w = S(#z)

/* 0+y=w -> y=w */
O :-: A(#y, #w)     => #y = #w

/* x'*y=w -> x*u=z, z+v=w, y=u & y=v */
S(#x) :-: M(#y, #w) => #x = M(#u, #z), #z = A(#v, #w), #y = D(#u, #v)

/* 0*y=w -> forall y, w=0 */
O :-: M(#y, #w)     => #y = E, #w = O

/* x'=u & x'=v -> u=y', v=z', x=y & x=z */
S(#x) :-: D(#u, #v) => #u = S(#y), #v = S(#z), #x = D(#y, #z)

/* 0=u & 0=v -> u=0, v=0 */
O :-: D(#u, #v)     => #u = O, #v = O

/* forall x' -> forall x */
S(#x) :-: E         => #x = E

/* forall 0 -> _ */
O :-: E             => _

/* 2*2=r */
S(S(O)) = M(S(S(O)), #r)

/* r=4 */
$ = #r
    "#;

    let program = zamuza::parser::parse(program)?;

    println!("/*\n{}*/", program);

    let mut runtime = zamuza::runtime::RuntimeBuilder::new();
    runtime.program(program)?;
    let runtime = runtime.build()?;
    println!("{}", runtime);

    Ok(())
}
