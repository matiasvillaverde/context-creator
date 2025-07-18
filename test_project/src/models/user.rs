pub struct User {
    pub id: u32,
    pub name: String,
}

impl User {
    pub fn find(id: u32) -> Self {
        Self { id, name: "Test User".to_string() }
    }
}