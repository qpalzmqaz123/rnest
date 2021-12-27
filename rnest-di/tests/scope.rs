use rnest_di::Di;

#[tokio::test]
async fn test_scope_01() {
    let di = Di::new();

    let di_a = di.scope("A", &["B"]);
    di_a.register_value("a", 1_i32, true).unwrap();

    let di_b = di.scope("B", &[]);
    di_b.register_value("b1", 2_i32, true).unwrap();
    di_b.register_value("b2", 3_i32, false).unwrap();

    let di_a = di.scope("A", &["B"]);
    assert_eq!(1, di_a.inject("a").await.unwrap());
    assert_eq!(2, di_a.inject("b1").await.unwrap());
    assert!((di_a.inject("b2").await as rnest_di::Result<i32>).is_err());
}

#[tokio::test]
async fn test_scope_02() {
    let di = Di::new();

    let di_a = di.scope("A", &["B"]);
    di_a.register_factory(
        "a",
        |di| async move { Ok(di.inject::<i32>("b1").await? + di.inject::<i32>("b2").await?) },
        true,
    )
    .unwrap();

    let di_b = di.scope("B", &[]);
    di_b.register_factory("b1", |_| async { Ok(2_i32) }, true)
        .unwrap();
    di_b.register_factory("b2", |_| async { Ok(3_i32) }, true)
        .unwrap();
    di_b.register_factory("b3", |_| async { Ok(4_i32) }, false)
        .unwrap();

    let di_c = di.scope("C", &["B"]);
    di_c.register_factory("c1", |_| async { Ok(5_i32) }, false)
        .unwrap();
    di_c.register_factory(
        "c2",
        |di| async move { Ok(di.inject::<i32>("c1").await?) },
        true,
    )
    .unwrap();
    di_c.register_factory(
        "c3",
        |di| async move { Ok(di.inject::<i32>("b1").await?) },
        true,
    )
    .unwrap();
    di_c.register_factory(
        "c4",
        |di| async move { Ok(di.inject::<i32>("b3").await?) },
        true,
    )
    .unwrap();

    let di_a = di.scope("A", &["B"]);
    assert_eq!(5, di_a.inject("a").await.unwrap());
    assert_eq!(2, di_a.inject("b1").await.unwrap());
    assert_eq!(3, di_a.inject("b2").await.unwrap());
    assert!((di_a.inject("b3").await as rnest_di::Result<i32>).is_err());

    let di_b = di.scope("B", &[]);
    assert_eq!(4, di_b.inject("b3").await.unwrap());

    let di_c = di.scope("C", &["B"]);
    assert_eq!(5, di_c.inject("c1").await.unwrap());
    assert_eq!(5, di_c.inject("c2").await.unwrap());
    assert_eq!(2, di_c.inject("c3").await.unwrap());
    assert!((di_c.inject("c4").await as rnest_di::Result<i32>).is_err());
}

#[tokio::test]
async fn test_circulation() {
    let di = Di::new();

    di.scope("A", &["B"])
        .register_factory(
            "a",
            |di| async move { Ok(di.inject::<i32>("b").await?) },
            true,
        )
        .unwrap();
    di.scope("B", &["A"])
        .register_factory(
            "b",
            |di| async move { Ok(di.inject::<i32>("a").await?) },
            true,
        )
        .unwrap();

    let a = di.scope("A", &["B"]).inject::<i32>("a").await;
    assert!(a.is_err());
}
