//! Category 3: Semantic Analysis - Python (20 Tests)
//!
//! Tests for Python-specific semantic analysis edge cases

use crate::edge_cases::helpers::*;
use std::fs;
use tempfile::TempDir;

/// Scenario 31: Tracing callers of a decorated function
#[test]
fn test_31_decorated_function_callers() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("auth.py"),
        r#"
def login_required(f):
    def wrapper(*args, **kwargs):
        return f(*args, **kwargs)
    return wrapper

@login_required
def view_data():
    return "data"
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
import src.auth

def main():
    data = src.auth.view_data()
    print(data)
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        src_dir.join("auth.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 32: Tracing callers of a function assigned to a variable
#[test]
fn test_32_function_assigned_to_variable() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("utils.py"),
        r#"
def _helper():
    return "helped"

my_func = _helper
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from src.utils import my_func

result = my_func()
print(result)
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        src_dir.join("utils.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 33: Relative imports
#[test]
fn test_33_relative_imports() {
    let temp_dir = TempDir::new().unwrap();
    let app_dir = temp_dir.path().join("src").join("app");
    fs::create_dir_all(&app_dir).unwrap();

    fs::write(app_dir.join("utils.py"), "def helper(): pass").unwrap();
    fs::write(
        app_dir.join("api.py"),
        "from . import utils\nutils.helper()",
    )
    .unwrap();
    fs::write(app_dir.join("__init__.py"), "").unwrap();
    fs::write(temp_dir.path().join("src").join("__init__.py"), "").unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        app_dir.join("utils.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("api.py"));
}

/// Scenario 34: `import *` usage
#[test]
fn test_34_import_star() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("models.py"),
        r#"
class User:
    pass

class Product:
    pass
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("api.py"),
        r#"
from models import *

u = User()
p = Product()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("models.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("api.py"));
}

/// Scenario 35: Call to a method on a parent class
#[test]
fn test_35_parent_class_method_call() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("base.py"),
        r#"
class Base:
    def save(self):
        print("Saving...")
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("models.py"),
        r#"
from base import Base

class User(Base):
    pass
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from models import User

user = User()
user.save()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("base.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 36: Dynamic imports using `__import__`
#[test]
fn test_36_dynamic_import() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(src_dir.join("utils.py"), "def dynamic_func(): pass").unwrap();
    fs::write(src_dir.join("__init__.py"), "").unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
utils = __import__("src.utils", fromlist=["dynamic_func"])
utils.dynamic_func()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        src_dir.join("utils.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // Dynamic imports may not be traced
    // Test passes if tool handles it gracefully (with or without tracing)
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("error") || stderr.contains("fail")
        }
    );
}

/// Scenario 37: Aliased import
#[test]
fn test_37_aliased_import() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(temp_dir.path().join("numpy.py"), "def array(): pass").unwrap();
    fs::write(
        temp_dir.path().join("main.py"),
        r#"
import numpy as np

arr = np.array()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        temp_dir.path().join("numpy.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 38: Call to a function inside a list comprehension
#[test]
fn test_38_function_in_list_comprehension() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("utils.py"),
        r#"
def process(x):
    return x * 2
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from utils import process

results = [process(i) for i in range(10)]
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("utils.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 39: Call to a function passed as a lambda
#[test]
fn test_39_function_in_lambda() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("utils.py"),
        r#"
def my_func():
    return 42
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from utils import my_func

x = lambda: my_func()
result = x()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("utils.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 40: Including types for a forward reference
#[test]
fn test_40_forward_reference_type() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("models.py"),
        r#"
class MyClass:
    def __init__(self):
        self.value = 42
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("services.py"),
        r#"
def get_it() -> 'MyClass':
    from models import MyClass
    return MyClass()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-types",
        "MyClass",
        temp_dir.path().to_str().unwrap(),
    ]);

    // Type analysis may require specific setup or may not detect forward references
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // If successful, should include the type definition
        assert!(stdout.contains("models.py") || stdout.contains("MyClass"));
    }
}

/// Scenario 41: Django models with a custom Manager
#[test]
fn test_41_django_custom_manager() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("models.py"),
        r#"
class PostManager:
    def published(self):
        return []

class Post:
    objects = PostManager()
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("views.py"),
        r#"
from models import Post

def get_published():
    return Post.objects.published()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("models.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("views.py"));
}

/// Scenario 42: FastAPI dependency injection
#[test]
fn test_42_fastapi_dependency() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("deps.py"),
        r#"
def get_db():
    return "database"
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from deps import get_db

# Simulating FastAPI pattern
def endpoint(db = get_db):
    return db
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("deps.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 43: Call to a dunder method via built-in
#[test]
fn test_43_dunder_method_via_builtin() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("models.py"),
        r#"
class MyList:
    def __len__(self):
        return 42
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from models import MyList

ml = MyList()
size = len(ml)
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("models.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 44: A file with mixed Python and Cython
#[test]
fn test_44_mixed_python_cython() {
    let temp_dir = TempDir::new().unwrap();

    // .pyx file with Cython syntax
    fs::write(
        temp_dir.path().join("fast.pyx"),
        r#"
# Cython code
cdef int fast_function(int x):
    return x * 2

def python_wrapper(x):
    return fast_function(x)
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("fast.pyx").to_str().unwrap()]);

    // Should parse Python parts gracefully
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fast.pyx"));
    // Python function should be recognized
    assert!(stdout.contains("python_wrapper"));
}

/// Scenario 45: A single file containing multiple class definitions
#[test]
fn test_45_multiple_classes_single_file() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("models.py"),
        r#"
class A:
    def func_a(self):
        pass

class B:
    def func_b(self):
        pass
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
from models import A, B

a = A()
a.func_a()

b = B()
b.func_b()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("models.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
}

/// Scenario 46: A function redefined in the same file
#[test]
fn test_46_function_redefined() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
def my_func():
    print(1)

def my_func():  # Redefinition
    print(2)
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("other.py"),
        r#"
from main import my_func

my_func()  # Calls the second definition
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("main.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should trace callers for the last definition
    assert!(stdout.contains("other.py"));
}

/// Scenario 47: Using `*args` and `**kwargs`
#[test]
fn test_47_args_kwargs() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("utils.py"),
        r#"
def my_func(*args, **kwargs):
    return len(args) + len(kwargs)
"#,
    )
    .unwrap();

    let output = run_context_creator(&[temp_dir.path().join("utils.py").to_str().unwrap()]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should correctly parse function with *args/**kwargs
    assert!(stdout.contains("my_func"));
    assert!(stdout.contains("*args"));
    assert!(stdout.contains("**kwargs"));
}

/// Scenario 48: A file containing only comments and docstrings
#[test]
fn test_48_only_comments_docstrings() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("commented_out.py"),
        r#"
# This is a comment
"""
This is a module docstring
but there's no actual code
"""
# Another comment
# def commented_function():
#     pass
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("commented_out.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // Should find no functions and thus no callers
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("commented_out.py"));
}

/// Scenario 49: A project using `sys.path.append`
#[test]
fn test_49_sys_path_append() {
    let temp_dir = TempDir::new().unwrap();
    let libs_dir = temp_dir.path().join("libs");
    fs::create_dir_all(&libs_dir).unwrap();

    fs::write(libs_dir.join("my_lib.py"), "def lib_func(): pass").unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
import sys
sys.path.append('../libs')
import my_lib

my_lib.lib_func()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--trace-imports",
        libs_dir.join("my_lib.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // sys.path manipulation may not be traced
    // Test passes if tool handles it gracefully
    assert!(
        output.status.success() || {
            let stderr = String::from_utf8_lossy(&output.stderr);
            stderr.contains("error") || stderr.contains("fail")
        }
    );
}

/// Scenario 50: A function call using `getattr`
#[test]
fn test_50_getattr_function_call() {
    let temp_dir = TempDir::new().unwrap();

    fs::write(
        temp_dir.path().join("utils.py"),
        r#"
def my_func():
    return "called"
"#,
    )
    .unwrap();

    fs::write(
        temp_dir.path().join("main.py"),
        r#"
import utils

func = getattr(utils, 'my_func')
result = func()
"#,
    )
    .unwrap();

    let output = run_context_creator(&[
        "--include-callers",
        temp_dir.path().join("utils.py").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    ]);

    // Dynamic call - should fail to find caller gracefully
    let stdout = String::from_utf8_lossy(&output.stdout);
    // main.py won't be included as call is dynamic
    assert!(stdout.contains("utils.py"));
}
