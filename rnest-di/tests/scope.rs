use rnest_di::Di;

#[test]
fn test_scope_01() {
    let mut di = Di::new();

    let mut di_a = di.scope("A", &["B"]);
    di_a.register_value("a", 1_i32, true);

    let mut di_b = di.scope("B", &[]);
    di_b.register_value("b1", 2_i32, true);
    di_b.register_value("b2", 3_i32, false);

    let mut di_a = di.scope("A", &["B"]);
    assert_eq!(1, di_a.inject("a").unwrap());
    assert_eq!(2, di_a.inject("b1").unwrap());
    assert!((di_a.inject("b2") as rnest_error::Result<i32>).is_err());
}

#[test]
fn test_scope_02() {
    let mut di = Di::new();

    let mut di_a = di.scope("A", &["B"]);
    di_a.register_factory(
        "a",
        |di| Ok(di.inject::<_, i32>("b1")? + di.inject::<_, i32>("b2")?),
        true,
    );

    let mut di_b = di.scope("B", &[]);
    di_b.register_factory("b1", |_| Ok(2_i32), true);
    di_b.register_factory("b2", |_| Ok(3_i32), true);
    di_b.register_factory("b3", |_| Ok(4_i32), false);

    let mut di_a = di.scope("A", &["B"]);
    assert_eq!(5, di_a.inject("a").unwrap());
    assert_eq!(2, di_a.inject("b1").unwrap());
    assert_eq!(3, di_a.inject("b2").unwrap());
    assert!((di_a.inject("b3") as rnest_error::Result<i32>).is_err());
}

#[test]
fn test_circulation() {
    let mut di = Di::new();

    di.scope("A", &["B"])
        .register_factory("a", |di| Ok(di.inject::<_, i32>("b")?), true);
    di.scope("B", &["A"])
        .register_factory("b", |di| Ok(di.inject::<_, i32>("a")?), true);

    let a = di.scope("A", &["B"]).inject::<_, i32>("a");
    assert!(a.is_err());
}
