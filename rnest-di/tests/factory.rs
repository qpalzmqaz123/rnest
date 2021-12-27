use std::sync::Arc;

use rnest_di::Di;

struct Person {
    name: String,
}

impl Person {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }

    pub fn speak(&self) -> String {
        format!("My name is {}", self.name)
    }
}

#[tokio::test]
async fn test_factory() {
    let di = Di::new();

    di.register_factory("bob", |_| async { Ok(Arc::new(Person::new("bob"))) })
        .unwrap();

    let person: Arc<Person> = di.inject("bob").await.unwrap();

    assert_eq!(person.speak(), "My name is bob");
}
