#![cfg(test)]
use crate::datatypes::{Secret, User};

#[test]
fn test_type_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<User>(); // Wird nicht kompilieren wenn YourType nicht Send ist
}

// Alternative mit thread::spawn
#[test]
fn test_send_with_thread() {
    let value = User {
        id: 1,
        username: "hi".to_string(),
        email: "emil".to_string(),
        password: Secret::new("none".to_string()),
        role: crate::apirequests::users::Role::Admin,
        language: crate::datatypes::Lang::DeDE,
    };
    std::thread::spawn(move || {
        // Wenn dies kompiliert, ist der Typ Send
        drop(value);
    });
}
