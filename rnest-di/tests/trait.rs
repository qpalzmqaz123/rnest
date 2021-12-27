use std::sync::Arc;

use rnest_di::Di;

trait Human: Send + Sync {
    fn speak(&self) -> String;
}

struct Person {
    name: String,
}

impl Person {
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self { name: name.into() }
    }
}

impl Human for Person {
    fn speak(&self) -> String {
        format!("My name is {}", self.name)
    }
}

#[tokio::test]
async fn test_trait() {
    let di = Di::new();

    di.register_factory("bob", |_| async {
        Ok(Arc::new(Person::new("bob")) as Arc<dyn Human>)
    })
    .unwrap();

    let person: Arc<dyn Human> = di.inject("bob").await.unwrap();

    assert_eq!(person.speak(), "My name is bob");
}
