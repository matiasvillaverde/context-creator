//! Sample Rust project for testing context-creator

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse<T> {
    data: T,
    status: String,
}

/// Main entry point
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Sample Rust Project");
    
    // Example user
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    
    // Simulate API response
    let response = ApiResponse {
        data: user,
        status: "success".to_string(),
    };
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&response)?;
    println!("Response: {}", json);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User {
            id: 1,
            name: "Test".to_string(),
            email: "test@example.com".to_string(),
        };
        
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Test");
    }
}