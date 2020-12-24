use std::rc::Rc;

use rnest_di::Di;

trait Human {
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

#[test]
fn test_trait() {
    let mut di = Di::new();

    di.register_factory("bob", |_| Ok(Rc::new(Person::new("bob")) as Rc<dyn Human>));

    let person: Rc<dyn Human> = di.inject("bob").unwrap();

    assert_eq!(person.speak(), "My name is bob");
}
