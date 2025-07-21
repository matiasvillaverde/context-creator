//! Category 5: Semantic Analysis - Rust (15 Tests)
//!
//! Tests for Rust-specific semantic analysis edge cases

use crate::edge_cases::helpers::*;
use std::fs;
use tempfile::TempDir;

/// Scenario 71: Tracing trait implementations
#[test]
fn test_71_rust_trait_implementations() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("traits.rs"),
        r#"
pub trait Display {
    fn fmt(&self) -> String;
}

pub trait Debug {
    fn debug(&self) -> String;
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("models.rs"),
        r#"
use crate::traits::{Display, Debug};

pub struct User {
    pub name: String,
}

impl Display for User {
    fn fmt(&self) -> String {
        format!("User: {}", self.name)
    }
}

impl Debug for User {
    fn debug(&self) -> String {
        format!("User {{ name: {:?} }}", self.name)
    }
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("lib.rs"),
        r#"
pub mod traits;
pub mod models;
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include",
        "models.rs",
        "--include-types",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("models.rs"));
}

/// Scenario 72: Rust macro usage and expansion
#[test]
fn test_72_rust_macro_usage() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("macros.rs"),
        r#"
#[macro_export]
macro_rules! create_function {
    ($func_name:ident) => {
        fn $func_name() {
            println!("Function {} was called", stringify!($func_name));
        }
    };
}

#[macro_export]
macro_rules! impl_trait {
    ($type:ty) => {
        impl MyTrait for $type {
            fn method(&self) {}
        }
    };
}
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.rs"),
        r#"
#[macro_use]
extern crate macros;

create_function!(hello);
create_function!(world);

trait MyTrait {
    fn method(&self);
}

struct MyStruct;
impl_trait!(MyStruct);

fn main() {
    hello();
    world();
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("macros.rs").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // Macro expansion may not be fully traced
    assert!(output.status.success());
}

/// Scenario 73: Rust async trait methods
#[test]
fn test_73_async_trait_methods() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("async_traits.rs"),
        r#"
use async_trait::async_trait;

#[async_trait]
pub trait AsyncProcessor {
    async fn process(&self, data: &str) -> String;
}

pub struct MyProcessor;

#[async_trait]
impl AsyncProcessor for MyProcessor {
    async fn process(&self, data: &str) -> String {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        data.to_uppercase()
    }
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("async_traits.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("AsyncProcessor"));
}

/// Scenario 74: Rust generic associated types (GATs)
#[test]
fn test_74_generic_associated_types() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("gats.rs"),
        r#"
pub trait Container {
    type Item<'a> where Self: 'a;
    
    fn get<'a>(&'a self) -> Self::Item<'a>;
}

pub struct MyContainer<T> {
    value: T,
}

impl<T> Container for MyContainer<T> {
    type Item<'a> = &'a T where Self: 'a;
    
    fn get<'a>(&'a self) -> Self::Item<'a> {
        &self.value
    }
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("gats.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Container"));
}

/// Scenario 75: Rust module re-exports and visibility
#[test]
fn test_75_module_reexports() {
    let temp_dir = TempDir::new().unwrap();
    let core_dir = temp_dir.path().join("core");
    fs::create_dir_all(&core_dir).unwrap();

    fs::write(
        core_dir.join("internal.rs"),
        r#"
pub struct InternalStruct {
    pub value: i32,
}

pub(crate) fn internal_function() -> i32 {
    42
}
"#,
    )
    .unwrap();

    fs::write(
        core_dir.join("mod.rs"),
        r#"
mod internal;

pub use internal::InternalStruct;
// internal_function is not re-exported
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("lib.rs"),
        r#"
pub mod core;

pub use core::InternalStruct;
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        core_dir.join("internal.rs").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mod.rs") || stdout.contains("lib.rs"));
}

/// Scenario 76: Rust lifetime parameters and bounds
#[test]
fn test_76_lifetime_parameters() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("lifetimes.rs"),
        r#"
pub struct Parser<'a> {
    input: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Parser { input }
    }
    
    pub fn parse<'b>(&'b self) -> &'b str 
    where 
        'a: 'b 
    {
        self.input
    }
}

pub fn longest<'a, 'b>(x: &'a str, y: &'b str) -> &'a str 
where 
    'b: 'a
{
    if x.len() > y.len() { x } else { y }
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("lifetimes.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parser"));
}

/// Scenario 77: Rust procedural macros (derive)
#[test]
fn test_77_procedural_macros() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("derive.rs"),
        r#"
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub name: String,
    pub value: i32,
}

#[derive(Default)]
pub struct Settings {
    pub config: Option<Config>,
}

// Custom derive
#[derive(MyCustomDerive)]
pub struct CustomStruct {
    field: String,
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("derive.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Config"));
}

/// Scenario 78: Rust const generics
#[test]
fn test_78_const_generics() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("const_generics.rs"),
        r#"
pub struct Array<T, const N: usize> {
    data: [T; N],
}

impl<T: Default, const N: usize> Array<T, N> {
    pub fn new() -> Self {
        Array {
            data: [(); N].map(|_| T::default()),
        }
    }
}

pub fn split_array<T, const N: usize>(arr: [T; N]) -> ([T; N/2], [T; N/2]) 
where 
    [T; N/2]: Sized,
{
    todo!()
}
"#,
    )
    .unwrap();

    let output =
        run_context_creator(&[temp_dir.path().join("const_generics.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Array"));
}

/// Scenario 79: Rust workspace with multiple crates
#[test]
fn test_79_workspace_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let core_crate = temp_dir.path().join("core");
    let app_crate = temp_dir.path().join("app");

    fs::create_dir_all(core_crate.join("src")).unwrap();
    fs::create_dir_all(app_crate.join("src")).unwrap();

    // Workspace Cargo.toml
    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"
[workspace]
members = ["core", "app"]
"#,
    )
    .unwrap();

    // Core crate
    fs::write(
        core_crate.join("Cargo.toml"),
        r#"
[package]
name = "core"
version = "0.1.0"
"#,
    )
    .unwrap();

    fs::write(
        core_crate.join("src/lib.rs"),
        r#"
pub fn core_function() -> String {
    "Hello from core".to_string()
}
"#,
    )
    .unwrap();

    // App crate
    fs::write(
        app_crate.join("Cargo.toml"),
        r#"
[package]
name = "app"
version = "0.1.0"

[dependencies]
core = { path = "../core" }
"#,
    )
    .unwrap();

    fs::write(
        app_crate.join("src/main.rs"),
        r#"
use core::core_function;

fn main() {
    println!("{}", core_function());
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        core_crate.join("src/lib.rs").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // Cross-crate dependencies may not be fully traced
    assert!(output.status.success());
}

/// Scenario 80: Rust unsafe blocks and functions
#[test]
fn test_80_unsafe_code() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("unsafe_code.rs"),
        r#"
pub unsafe fn dangerous_function(ptr: *const i32) -> i32 {
    *ptr
}

pub struct RawWrapper {
    ptr: *mut u8,
}

impl RawWrapper {
    pub unsafe fn new(ptr: *mut u8) -> Self {
        RawWrapper { ptr }
    }
    
    pub fn safe_method(&self) {
        unsafe {
            // Unsafe operations here
            let _ = self.ptr;
        }
    }
}

pub fn use_unsafe() {
    let value = 42;
    let result = unsafe { dangerous_function(&value) };
    println!("{}", result);
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("unsafe_code.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dangerous_function"));
}

/// Scenario 81: Rust pattern matching with guards
#[test]
fn test_81_pattern_matching() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("patterns.rs"),
        r#"
pub enum Message {
    Text(String),
    Number(i32),
    Tuple(String, i32),
}

pub fn process_message(msg: Message) -> String {
    match msg {
        Message::Text(s) if s.len() > 10 => format!("Long text: {}", s),
        Message::Text(s) => format!("Short text: {}", s),
        Message::Number(n) if n < 0 => format!("Negative: {}", n),
        Message::Number(n) => format!("Positive: {}", n),
        Message::Tuple(s, n) if n == 0 => format!("Zero tuple: {}", s),
        Message::Tuple(s, n) => format!("Tuple: {} {}", s, n),
    }
}

pub fn destructure_complex(value: &[(i32, Option<String>)]) {
    for (num, maybe_string) in value {
        match (num, maybe_string) {
            (0..=10, Some(s)) if s.starts_with("A") => {},
            (n, None) if *n % 2 == 0 => {},
            _ => {},
        }
    }
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("patterns.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("process_message"));
}

/// Scenario 82: Rust type aliases and newtype patterns
#[test]
fn test_82_type_aliases_newtypes() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("types.rs"),
        r#"
pub type Result<T> = std::result::Result<T, Error>;
pub type HashMap<K, V> = std::collections::HashMap<K, V>;

#[derive(Debug)]
pub struct Error(String);

// Newtype pattern
pub struct UserId(pub u64);
pub struct Email(pub String);

impl UserId {
    pub fn new(id: u64) -> Self {
        UserId(id)
    }
}

impl From<String> for Email {
    fn from(s: String) -> Self {
        Email(s)
    }
}

pub fn process_user(id: UserId, email: Email) -> Result<()> {
    Ok(())
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("types.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("UserId"));
}

/// Scenario 83: Rust closures with move semantics
#[test]
fn test_83_closures_move_semantics() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("closures.rs"),
        r#"
pub fn create_closure() -> impl Fn() -> String {
    let captured = String::from("Hello");
    move || captured.clone()
}

pub fn higher_order_function<F>(f: F) -> String 
where 
    F: Fn(i32) -> i32
{
    format!("Result: {}", f(42))
}

pub struct EventHandler<F> 
where 
    F: Fn(&str) + Send + Sync + 'static
{
    handler: F,
}

impl<F> EventHandler<F> 
where 
    F: Fn(&str) + Send + Sync + 'static
{
    pub fn new(handler: F) -> Self {
        EventHandler { handler }
    }
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("closures.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("create_closure"));
}

/// Scenario 84: Rust impl blocks with where clauses
#[test]
fn test_84_impl_where_clauses() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("where_clauses.rs"),
        r#"
use std::fmt::Display;

pub struct Container<T> {
    value: T,
}

impl<T> Container<T> {
    pub fn new(value: T) -> Self {
        Container { value }
    }
}

impl<T> Container<T> 
where 
    T: Display
{
    pub fn display(&self) -> String {
        format!("Container: {}", self.value)
    }
}

impl<T> Container<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn clone_value(&self) -> T {
        self.value.clone()
    }
}

pub trait MyTrait {
    type Item;
}

impl<T> MyTrait for Container<T>
where
    T: Display + Default,
{
    type Item = T;
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("where_clauses.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Container"));
}

/// Scenario 85: Rust external crate imports with features
#[test]
fn test_85_external_crate_features() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("external.rs"),
        r#"
#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

#[cfg(feature = "async")]
use tokio::runtime::Runtime;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Config {
    pub name: String,
    #[cfg(feature = "advanced")]
    pub advanced_option: Option<String>,
}

#[cfg(all(feature = "async", feature = "client"))]
pub async fn async_client_function() {
    // Async client code
}

#[cfg(any(feature = "json", feature = "yaml"))]
pub fn parse_config(data: &str) -> Config {
    todo!()
}
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("external.rs").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Config"));
}
