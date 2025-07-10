//! Library module for the sample project

pub mod utils;
pub mod api;

pub use api::{Client, Error};

/// Configuration for the application
#[derive(Debug, Clone)]
pub struct Config {
    pub api_url: String,
    pub timeout: u64,
    pub max_retries: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: "https://api.example.com".to_string(),
            timeout: 30,
            max_retries: 3,
        }
    }
}