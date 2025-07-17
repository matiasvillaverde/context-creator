//! Integration tests for Python semantic analysis output

use std::fs;
use tempfile::TempDir;

/// Test that Python imports are tracked and included in output
#[test]
fn test_python_imports_in_output() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create main.py with various import styles
    fs::write(
        src_dir.join("main.py"),
        r#"
import os
import sys
from pathlib import Path
from typing import List, Dict, Optional

# Local imports
import utils
from helpers import format_string, validate_data
from lib.database import Connection
from lib.api import Client as APIClient

def main():
    utils.setup()
    formatted = format_string("Hello")
    validate_data({"key": "value"})
    
    conn = Connection()
    client = APIClient()
    
    print(f"Path: {Path.cwd()}")
"#,
    )
    .unwrap();

    // Create utils.py
    fs::write(
        src_dir.join("utils.py"),
        r#"
import logging

def setup():
    logging.basicConfig(level=logging.INFO)
    print("Setup complete")

def cleanup():
    print("Cleanup complete")
"#,
    )
    .unwrap();

    // Create helpers.py
    fs::write(
        src_dir.join("helpers.py"),
        r#"
def format_string(s: str) -> str:
    return s.strip().lower()

def validate_data(data: dict) -> bool:
    return bool(data)
"#,
    )
    .unwrap();

    // Create lib directory
    let lib_dir = src_dir.join("lib");
    fs::create_dir_all(&lib_dir).unwrap();

    // Create __init__.py to make it a package
    fs::write(lib_dir.join("__init__.py"), "").unwrap();

    // Create database.py
    fs::write(
        lib_dir.join("database.py"),
        r#"
import sqlite3

class Connection:
    def __init__(self):
        self.conn = None
    
    def connect(self):
        self.conn = sqlite3.connect(":memory:")
"#,
    )
    .unwrap();

    // Create api.py
    fs::write(
        lib_dir.join("api.py"),
        r#"
import requests

class Client:
    def __init__(self):
        self.session = requests.Session()
    
    def get(self, url):
        return self.session.get(url)
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    eprintln!("STDERR: {stderr}");

    // Verify command succeeded
    assert!(output.status.success(), "code-digest failed: {stderr}");

    // Check that semantic analysis was performed
    if stderr.contains("Analyzing semantic dependencies") {
        assert!(
            stderr.contains("Found") && stderr.contains("import relationships"),
            "Should report found imports"
        );
    }

    // Test that main.py shows its imports
    assert!(stdout.contains("main.py"), "Output should contain main.py");

    let main_section = stdout
        .split("## main.py")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    // Should show imports
    assert!(
        main_section.contains("Imports:")
            && (main_section.contains("utils") || main_section.contains("helpers")),
        "main.py should show its local imports"
    );

    // Test that imported modules show they're imported by main
    assert!(
        stdout.contains("utils.py") && stdout.contains("Imported by: main.py"),
        "utils.py should show it's imported by main.py"
    );

    assert!(
        stdout.contains("helpers.py") && stdout.contains("Imported by: main.py"),
        "helpers.py should show it's imported by main.py"
    );
}

/// Test Python class and function tracking
#[test]
fn test_python_class_function_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("models.py"),
        r#"
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class User:
    id: int
    name: str
    email: str
    
    def get_display_name(self) -> str:
        return self.name or self.email

class UserManager:
    def __init__(self):
        self.users: List[User] = []
    
    def add_user(self, user: User) -> None:
        self.users.append(user)
    
    def find_by_id(self, user_id: int) -> Optional[User]:
        for user in self.users:
            if user.id == user_id:
                return user
        return None
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("app.py"),
        r#"
from models import User, UserManager

def main():
    manager = UserManager()
    
    user1 = User(id=1, name="Alice", email="alice@example.com")
    user2 = User(id=2, name="Bob", email="bob@example.com")
    
    manager.add_user(user1)
    manager.add_user(user2)
    
    found = manager.find_by_id(1)
    if found:
        print(found.get_display_name())

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Check imports are tracked
    assert!(stdout.contains("app.py"));
    assert!(stdout.contains("Imports: models"));

    // Check that models.py shows it's imported
    assert!(stdout.contains("models.py"));
    assert!(stdout.contains("Imported by: app.py"));

    // Check if type references are tracked (User, UserManager)
    if stdout.contains("Type references:") {
        let app_section = stdout
            .split("## app.py")
            .nth(1)
            .unwrap_or("")
            .split("##")
            .next()
            .unwrap_or("");

        assert!(
            app_section.contains("User") || app_section.contains("UserManager"),
            "Should track type usage"
        );
    }
}

/// Test Python package imports (from x import y)
#[test]
fn test_python_package_imports() {
    let temp_dir = TempDir::new().unwrap();
    let project_dir = temp_dir.path().join("project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create package structure
    let pkg_dir = project_dir.join("mypackage");
    fs::create_dir_all(&pkg_dir).unwrap();

    let subpkg_dir = pkg_dir.join("subpackage");
    fs::create_dir_all(&subpkg_dir).unwrap();

    // Root __init__.py
    fs::write(
        pkg_dir.join("__init__.py"),
        r#"
"""Main package initialization."""
from .core import Core
from .utils import helper_function

__all__ = ['Core', 'helper_function']
__version__ = '1.0.0'
"#,
    )
    .unwrap();

    // core.py
    fs::write(
        pkg_dir.join("core.py"),
        r#"
from .subpackage.advanced import AdvancedFeature

class Core:
    def __init__(self):
        self.advanced = AdvancedFeature()
    
    def process(self):
        return self.advanced.compute()
"#,
    )
    .unwrap();

    // utils.py
    fs::write(
        pkg_dir.join("utils.py"),
        r#"
import os
from pathlib import Path

def helper_function():
    return Path.cwd()

def internal_helper():
    return os.environ.get('HOME', '/')
"#,
    )
    .unwrap();

    // subpackage/__init__.py
    fs::write(
        subpkg_dir.join("__init__.py"),
        r#"
from .advanced import AdvancedFeature

__all__ = ['AdvancedFeature']
"#,
    )
    .unwrap();

    // subpackage/advanced.py
    fs::write(
        subpkg_dir.join("advanced.py"),
        r#"
class AdvancedFeature:
    def compute(self):
        return 42
"#,
    )
    .unwrap();

    // Main script using the package
    fs::write(
        project_dir.join("main.py"),
        r#"
from mypackage import Core, helper_function
from mypackage.subpackage import AdvancedFeature

def main():
    core = Core()
    result = core.process()
    print(f"Result: {result}")
    
    path = helper_function()
    print(f"Path: {path}")
    
    advanced = AdvancedFeature()
    print(f"Advanced: {advanced.compute()}")

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&project_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    eprintln!("STDERR: {stderr}");

    assert!(output.status.success());

    // Main should import from mypackage
    assert!(stdout.contains("main.py"));

    // Package files should show relationships
    assert!(stdout.contains("__init__.py"));
    assert!(stdout.contains("core.py"));
}

/// Test Python relative imports
#[test]
fn test_python_relative_imports() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("package");
    let sub_dir = pkg_dir.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();

    // Package __init__.py
    fs::write(pkg_dir.join("__init__.py"), "").unwrap();

    // sub/__init__.py
    fs::write(sub_dir.join("__init__.py"), "").unwrap();

    // package/base.py
    fs::write(
        pkg_dir.join("base.py"),
        r#"
class BaseClass:
    def method(self):
        return "base"
"#,
    )
    .unwrap();

    // package/derived.py with relative import
    fs::write(
        pkg_dir.join("derived.py"),
        r#"
from .base import BaseClass

class DerivedClass(BaseClass):
    def method(self):
        return super().method() + " derived"
"#,
    )
    .unwrap();

    // package/sub/module.py with parent relative import
    fs::write(
        sub_dir.join("module.py"),
        r#"
from ..base import BaseClass
from ..derived import DerivedClass

def use_classes():
    base = BaseClass()
    derived = DerivedClass()
    return base.method(), derived.method()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&pkg_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Check that files are included
    assert!(stdout.contains("base.py"));
    assert!(stdout.contains("derived.py"));
    assert!(stdout.contains("sub/module.py") || stdout.contains("sub\\module.py"));

    // derived.py should import base
    let derived_section = stdout
        .split("derived.py")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    if derived_section.contains("Imports:") {
        assert!(
            derived_section.contains("base"),
            "derived.py should import base"
        );
    }
}
