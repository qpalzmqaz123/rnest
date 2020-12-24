use std::rc::Rc;

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

#[test]
fn test_factory() {
    let mut di = Di::new();

    di.register_factory("bob", |_| Ok(Rc::new(Person::new("bob"))));

    let person: Rc<Person> = di.inject("bob").unwrap();

    assert_eq!(person.speak(), "My name is bob");
}
