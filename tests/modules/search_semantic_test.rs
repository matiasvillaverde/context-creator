//! Tests for search command with semantic analysis

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_search_includes_types_not_matching_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with login function that uses Account type
    fs::write(
        temp_dir.path().join("auth.rs"),
        r#"
use crate::models::Account;

pub fn login(account: Account) -> bool {
    account.verify_credentials()
}
"#,
    )
    .unwrap();

    // Create Account type that doesn't contain "login"
    fs::write(
        temp_dir.path().join("models.rs"),
        r#"
pub struct Account {
    username: String,
    password_hash: String,
}

impl Account {
    pub fn verify_credentials(&self) -> bool {
        // verification logic
        true
    }
}
"#,
    )
    .unwrap();

    // Create another file that imports but doesn't match search
    fs::write(
        temp_dir.path().join("user.rs"),
        r#"
use crate::models::Account;

pub fn create_user(name: &str) -> Account {
    Account::new(name)
}
"#,
    )
    .unwrap();

    // Search for "login" - should include models.rs due to --include-types
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("login")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs")) // Direct match
        .stdout(predicate::str::contains("models.rs")) // Included due to Account type
        .stdout(predicate::str::contains("Account")) // The type definition
        .stdout(predicate::str::contains("verify_credentials")); // Method in Account
}

#[test]
fn test_search_includes_imports_not_matching_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file that matches search
    fs::write(
        temp_dir.path().join("payment.rs"),
        r#"
pub fn process_payment(amount: f64) -> Result<(), Error> {
    // payment logic
    Ok(())
}
"#,
    )
    .unwrap();

    // Create a file that imports payment but doesn't match search
    fs::write(
        temp_dir.path().join("checkout.rs"),
        r#"
use crate::payment::process_payment;

pub fn checkout(items: Vec<Item>) -> Result<(), Error> {
    let total = calculate_total(items);
    process_payment(total)
}
"#,
    )
    .unwrap();

    // Create another importer
    fs::write(
        temp_dir.path().join("api.rs"),
        r#"
use crate::checkout::checkout;

pub fn handle_checkout_request(request: Request) -> Response {
    match checkout(request.items) {
        Ok(_) => Response::success(),
        Err(e) => Response::error(e),
    }
}
"#,
    )
    .unwrap();

    // Search for "payment" - should include importers due to --trace-imports
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("payment")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("payment.rs")) // Direct match
        .stdout(predicate::str::contains("checkout.rs")) // Imports payment
        .stdout(predicate::str::contains("api.rs")); // Transitively imports payment
}

#[test]
fn test_search_includes_callers_not_matching_search() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with a specific function
    fs::write(
        temp_dir.path().join("validator.rs"),
        r#"
pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}
"#,
    )
    .unwrap();

    // Create files that call the function but don't match search
    fs::write(
        temp_dir.path().join("user_service.rs"),
        r#"
use crate::validator::validate_email;

pub fn register_user(email: &str, password: &str) -> Result<User, Error> {
    if !validate_email(email) {
        return Err(Error::InvalidEmail);
    }
    // registration logic
    Ok(User::new(email))
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("admin.rs"),
        r#"
use crate::validator::validate_email;

pub fn add_admin(email: &str) -> Result<Admin, Error> {
    if !validate_email(email) {
        return Err(Error::InvalidEmail);
    }
    Ok(Admin::new(email))
}
"#,
    )
    .unwrap();

    // Search for "validate_email" - should include callers
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("validate_email")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("validator.rs")) // Direct match
        .stdout(predicate::str::contains("user_service.rs")) // Calls validate_email
        .stdout(predicate::str::contains("admin.rs")) // Also calls validate_email
        .stdout(predicate::str::contains("register_user")) // Function that calls it
        .stdout(predicate::str::contains("add_admin")); // Another caller
}

#[test]
fn test_search_with_no_semantic_only_includes_matches() {
    let temp_dir = TempDir::new().unwrap();

    // Create interconnected files
    fs::write(
        temp_dir.path().join("auth.rs"),
        r#"
use crate::models::User;

pub fn authenticate(user: User) -> bool {
    user.is_valid()
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("models.rs"),
        r#"
pub struct User {
    id: u64,
}

impl User {
    pub fn is_valid(&self) -> bool {
        self.id > 0
    }
}
"#,
    )
    .unwrap();

    // Search with --no-semantic should only find direct matches
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("authenticate")
        .arg("--no-semantic")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.rs")) // Direct match
        .stdout(predicate::str::contains("models.rs").not()); // Should NOT include type
}

#[test]
fn test_search_semantic_follows_deep_chains() {
    let temp_dir = TempDir::new().unwrap();

    // Create a deep dependency chain
    fs::write(
        temp_dir.path().join("config.rs"),
        r#"
pub struct Config {
    pub api_key: String,
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("client.rs"),
        r#"
use crate::config::Config;

pub struct ApiClient {
    config: Config,
}

impl ApiClient {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("service.rs"),
        r#"
use crate::client::ApiClient;

pub struct PaymentService {
    client: ApiClient,
}

impl PaymentService {
    pub fn process_payment(&self, amount: f64) -> Result<(), Error> {
        // Uses client internally
        Ok(())
    }
}
"#,
    )
    .unwrap();

    // Search for "process_payment" should include the whole chain
    let mut cmd = Command::cargo_bin("context-creator").unwrap();
    cmd.arg("search")
        .arg("process_payment")
        .arg(temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("service.rs")) // Direct match
        .stdout(predicate::str::contains("client.rs")) // Type dependency
        .stdout(predicate::str::contains("config.rs")) // Transitive type dependency
        .stdout(predicate::str::contains("ApiClient")) // Intermediate type
        .stdout(predicate::str::contains("Config")); // Deep dependency
}
