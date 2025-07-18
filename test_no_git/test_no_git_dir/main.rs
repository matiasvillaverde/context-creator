use std::collections::HashMap;

mod models;
use models::User;

fn main() {
    let user = User { name: "test".to_string(), age: 30 };
    println!("{}", user.name);
}
