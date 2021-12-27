use rnest_di::Di;

#[tokio::test]
async fn test_value() {
    let di = Di::new();

    di.register_value("name", "bob".to_string()).unwrap();
    di.register_factory("hello", |di| async move {
        let name: String = di.inject("name").await?;
        Ok(format!("Hello {}", name))
    })
    .unwrap();
    di.register_factory("hi", |di| async move {
        let name: String = di.inject("name").await?;
        Ok(format!("Hi {}", name))
    })
    .unwrap();

    let hello_msg: String = di.inject("hello").await.unwrap();
    let hi_msg: String = di.inject("hi").await.unwrap();

    assert_eq!(hello_msg, "Hello bob".to_string());
    assert_eq!(hi_msg, "Hi bob".to_string());
    assert_eq!(hello_msg, di.inject_value::<String>("hello").unwrap());
    assert_eq!(hi_msg, di.inject_value::<String>("hi").unwrap());
}
