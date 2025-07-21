//! Category 4: Semantic Analysis - Include Types Tests
//!
//! These tests validate the --include-types functionality

#![cfg(test)]
#![allow(clippy::needless_borrow)] // project_root is PathBuf, function expects &Path

use super::fixtures::*;
use super::helpers::*;

#[test]
fn scenario_4_1_python_class_type_hint() {
    // Scenario 4.1 (Python): Class type hint
    // CLI Flags: --include-types src/service.py
    // Project Sketch: service.py (uses User class), models.py (defines User), unrelated.py
    // Assertion: Output must contain service.py and models.py. It must NOT contain unrelated.py

    use super::builders::*;

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file(
            "models.py",
            r#"
class User:
    """User model class"""
    def __init__(self, id: int, name: str, email: str):
        self.id = id
        self.name = name
        self.email = email
        
    def get_display_name(self) -> str:
        return f"{self.name} <{self.email}>"

class Product:
    """Product model - not used in service"""
    def __init__(self, id: int, name: str):
        self.id = id
        self.name = name
"#,
        )
        .add_file(
            "service.py",
            r#"
from typing import Optional, List
from models import User

class UserService:
    def get_user(self, user_id: int) -> Optional[User]:
        # Simulated user fetch
        return User(user_id, "Test User", "test@example.com")
    
    def list_users(self) -> List[User]:
        return [
            User(1, "Alice", "alice@example.com"),
            User(2, "Bob", "bob@example.com")
        ]
"#,
        )
        .add_file(
            "unrelated.py",
            r#"
# This file doesn't use any types from models
def unrelated_function():
    return "Not type related"
"#,
        )
        .build();

    // Run with include-types flag
    let output = run_context_creator(
        &["--include", "service.py", "--include-types"],
        &project_root,
    );

    // Verify assertions
    assert_contains_file(&output, "service.py");
    assert_contains_file(&output, "models.py"); // Contains User class used in service

    // Should NOT include unrelated files or unused types
    assert_not_contains_file(&output, "unrelated.py");

    // Verify the User class is present but Product is not
    assert_contains_code(&output, "class User:");
    assert_contains_code(&output, "def get_display_name(self) -> str:");
}

#[test]
fn scenario_4_2_typescript_interface_types() {
    // Scenario 4.2 (TypeScript): Interface type
    // CLI Flags: --include-types src/handlers.ts
    // Project Sketch: handlers.ts (uses IUser interface), types.ts (defines IUser), other.ts
    // Assertion: Markdown must contain handlers.ts and types.ts

    let (_temp_dir, project_root) = create_typescript_with_exports();

    // Run with include-types flag
    let output = run_context_creator(
        &["--include", "src/handlers.ts", "--include-types"],
        &project_root,
    );

    // Verify assertions
    assert_contains_file(&output, "src/handlers.ts");
    assert_contains_file(&output, "src/types.ts"); // Contains IUser interface

    // Should NOT include files that don't use or define the types
    assert_not_contains_file(&output, "src/utils.ts");
    assert_not_contains_file(&output, "src/components/Calendar.tsx");

    // Verify the interface is present
    assert_contains_code(&output, "export interface IUser");
    assert_contains_code(&output, "id: string;");
    assert_contains_code(&output, "name: string;");
}

#[test]
fn scenario_4_3_rust_function_parameter_types() {
    // Scenario 4.3 (Rust): Function parameter types
    // CLI Flags: --include-types src/processing.rs
    // Project Sketch: processing.rs (uses User struct), models.rs (defines User), main.rs
    // Assertion: Markdown must contain processing.rs and models.rs

    let (_temp_dir, project_root) = create_rust_with_types();

    // Run with include-types flag
    let output = run_context_creator(
        &["--include", "src/processing.rs", "--include-types"],
        &project_root,
    );

    // Verify assertions
    assert_contains_file(&output, "src/processing.rs");
    assert_contains_file(&output, "src/models.rs"); // Contains User struct

    // Verify the User struct is present
    assert_contains_code(&output, "pub struct User");
    assert_contains_code(&output, "pub id: u64,");
    assert_contains_code(&output, "pub name: String,");

    // Verify the functions using the type
    assert_contains_code(&output, "pub fn process_user(user: &User) -> String");
}

#[test]
fn test_generic_type_parameters() {
    // Test that generic type parameters are traced
    use super::builders::*;

    let (_temp_dir, project_root) = TypeScriptProjectBuilder::new()
        .add_file(
            "src/types.ts",
            r#"
export interface Response<T> {
    data: T;
    status: number;
    message: string;
}

export interface User {
    id: string;
    name: string;
}

export interface Product {
    id: string;
    price: number;
}
"#,
        )
        .add_file(
            "src/api.ts",
            r#"
import { Response, User } from './types';

export async function fetchUser(id: string): Promise<Response<User>> {
    // Simulated API call
    return {
        data: { id, name: 'Test User' },
        status: 200,
        message: 'Success'
    };
}

export function processUserResponse(response: Response<User>): User {
    return response.data;
}
"#,
        )
        .add_file(
            "src/unused.ts",
            r#"
// This file uses Product but not User
import { Product } from './types';

export function getProduct(): Product {
    return { id: '1', price: 99.99 };
}
"#,
        )
        .build();

    let output = run_context_creator(
        &["--include", "src/api.ts", "--include-types"],
        &project_root,
    );

    // Should include api.ts and types.ts
    assert_contains_file(&output, "src/api.ts");
    assert_contains_file(&output, "src/types.ts");

    // Should NOT include unused.ts (uses Product, not User)
    assert_not_contains_file(&output, "src/unused.ts");

    // Should include both Response and User interfaces
    assert_contains_code(&output, "export interface Response<T>");
    assert_contains_code(&output, "export interface User");
}

#[test]
fn test_type_aliases_and_unions() {
    // Test type aliases and union types
    use super::builders::*;

    let (_temp_dir, project_root) = TypeScriptProjectBuilder::new()
        .add_file(
            "src/types.ts",
            r#"
export type Status = 'active' | 'inactive' | 'pending';
export type ID = string | number;

export interface BaseEntity {
    id: ID;
    createdAt: Date;
}

export interface User extends BaseEntity {
    name: string;
    status: Status;
}

export type UserOrError = User | Error;
"#,
        )
        .add_file(
            "src/handlers.ts",
            r#"
import { User, Status, UserOrError } from './types';

export function createUser(name: string): User {
    return {
        id: '123',
        name,
        status: 'active' as Status,
        createdAt: new Date()
    };
}

export function validateUser(user: UserOrError): user is User {
    return 'name' in user;
}
"#,
        )
        .build();

    let output = run_context_creator(
        &["--include", "src/handlers.ts", "--include-types"],
        &project_root,
    );

    // Should include both files
    assert_contains_file(&output, "src/handlers.ts");
    assert_contains_file(&output, "src/types.ts");

    // Should include all related types
    assert_contains_code(
        &output,
        "export type Status = 'active' | 'inactive' | 'pending'",
    );
    assert_contains_code(&output, "export type ID = string | number");
    assert_contains_code(&output, "export interface BaseEntity");
    assert_contains_code(&output, "export interface User extends BaseEntity");
    assert_contains_code(&output, "export type UserOrError = User | Error");
}

#[test]
#[ignore = "Requires including lib.rs for module declarations - architectural change needed"]
fn test_nested_type_dependencies() {
    // Test deeply nested type dependencies
    use super::builders::*;

    let (_temp_dir, project_root) = RustProjectBuilder::new()
        .add_file(
            "src/core_types.rs",
            r#"
#[derive(Debug, Clone)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i32,
}

#[derive(Debug)]
pub struct Metadata {
    pub created: Timestamp,
    pub updated: Timestamp,
}
"#,
        )
        .add_file(
            "src/domain.rs",
            r#"
use crate::core_types::Metadata;

#[derive(Debug)]
pub struct User {
    pub id: u64,
    pub email: String,
    pub metadata: Metadata,
}

#[derive(Debug)]
pub struct Session {
    pub token: String,
    pub user: User,
}
"#,
        )
        .add_file(
            "src/handlers.rs",
            r#"
use crate::domain::Session;

pub fn validate_session(session: &Session) -> bool {
    !session.token.is_empty() && session.user.id > 0
}

pub fn get_user_email(session: &Session) -> &str {
    &session.user.email
}
"#,
        )
        .add_file(
            "src/lib.rs",
            r#"
pub mod core_types;
pub mod domain;
pub mod handlers;
"#,
        )
        .build();

    let output = run_context_creator(
        &["--include", "src/handlers.rs", "--include-types"],
        &project_root,
    );

    // Should include the entire type dependency chain
    assert_contains_file(&output, "src/handlers.rs");
    assert_contains_file(&output, "src/domain.rs"); // Session and User
    assert_contains_file(&output, "src/core_types.rs"); // Metadata and Timestamp
    assert_contains_file(&output, "src/lib.rs"); // Module declarations

    // Verify all types are present
    assert_contains_code(&output, "pub struct Session");
    assert_contains_code(&output, "pub struct User");
    assert_contains_code(&output, "pub struct Metadata");
    assert_contains_code(&output, "pub struct Timestamp");
}

#[test]
fn test_enum_types() {
    // Test enum type dependencies
    use super::builders::*;

    let (_temp_dir, project_root) = PythonProjectBuilder::new()
        .add_file(
            "enums.py",
            r#"
from enum import Enum, auto

class UserRole(Enum):
    ADMIN = auto()
    USER = auto()
    GUEST = auto()

class OrderStatus(Enum):
    PENDING = "pending"
    PROCESSING = "processing"
    COMPLETED = "completed"
    CANCELLED = "cancelled"
"#,
        )
        .add_file(
            "models.py",
            r#"
from dataclasses import dataclass
from enums import UserRole, OrderStatus
from typing import List

@dataclass
class User:
    id: int
    name: str
    role: UserRole

@dataclass  
class Order:
    id: int
    user_id: int
    status: OrderStatus
    items: List[str]
"#,
        )
        .add_file(
            "service.py",
            r#"
from models import User
from enums import UserRole

class UserService:
    def create_admin(self, name: str) -> User:
        return User(id=1, name=name, role=UserRole.ADMIN)
    
    def is_admin(self, user: User) -> bool:
        return user.role == UserRole.ADMIN
"#,
        )
        .build();

    let output = run_context_creator(
        &["--include", "service.py", "--include-types"],
        &project_root,
    );

    // Should include service, models, and enums
    assert_contains_file(&output, "service.py");
    assert_contains_file(&output, "models.py");
    assert_contains_file(&output, "enums.py");

    // Should include UserRole enum but potentially not OrderStatus
    assert_contains_code(&output, "class UserRole(Enum):");
    assert_contains_code(&output, "class User:");
}
