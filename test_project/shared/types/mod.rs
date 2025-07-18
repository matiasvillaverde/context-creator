pub struct ApiResponse<T> {
    pub data: T,
    pub status: u16,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self { data, status: 200 }
    }
}