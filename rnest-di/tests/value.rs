use rnest_di::Di;

#[test]
fn test_value() {
    let mut di = Di::new();

    di.register_value("name", "bob".to_string());
    di.register_factory("hello", |di| {
        let name: String = di.inject("name")?;
        Ok(format!("Hello {}", name))
    });
    di.register_factory("hi", |di| {
        let name: String = di.inject("name")?;
        Ok(format!("Hi {}", name))
    });

    let hello_msg: String = di.inject("hello").unwrap();
    let hi_msg: String = di.inject("hi").unwrap();

    assert_eq!(hello_msg, "Hello bob".to_string());
    assert_eq!(hi_msg, "Hi bob".to_string());
}
