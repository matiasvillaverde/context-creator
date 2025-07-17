//! Stress tests for Python semantic analysis

use std::fs;
use tempfile::TempDir;

/// Test Python with hundreds of imports
#[test]
fn test_python_massive_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create many modules
    let module_count = 200;
    for i in 0..module_count {
        fs::write(
            src_dir.join(format!("module_{i}.py")),
            format!(
                r#"
def function_{i}():
    return {i}

class Class{i}:
    value = {i}

CONSTANT_{i} = {i}
"#
            ),
        )
        .unwrap();
    }

    // Create main file that imports everything
    let mut content = String::new();

    // Add imports
    for i in 0..module_count {
        content.push_str(&format!(
            "from module_{i} import function_{i}, Class{i}, CONSTANT_{i}\n"
        ));
    }

    // Add main function that uses some imports
    content.push_str("\ndef main():\n    total = 0\n");
    for i in 0..10 {
        content.push_str(&format!("    total += function_{i}()\n"));
        content.push_str(&format!("    obj = Class{i}()\n"));
        content.push_str(&format!("    total += CONSTANT_{i}\n"));
    }
    content.push_str("    return total\n\nif __name__ == '__main__':\n    print(main())\n");

    fs::write(src_dir.join("main.py"), content).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .arg("--include-types")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success(), "Should handle 200+ imports");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("main.py"));
    assert!(stdout.contains("Imports:"));
}

/// Test Python with deeply nested function calls
#[test]
fn test_python_deep_call_chain() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create a chain of modules calling each other
    let depth = 50;

    for i in 0..depth {
        let content = if i == depth - 1 {
            // Last function in chain
            format!(
                r#"
def function_{i}(n):
    print(f"Reached depth {i}")
    return n + {i}
"#
            )
        } else {
            // Function that imports and calls the next one
            format!(
                r#"
from level_{} import function_{}

def function_{i}(n):
    return function_{}(n + {i})
"#,
                i + 1,
                i + 1,
                i + 1
            )
        };

        fs::write(src_dir.join(format!("level_{i}.py")), content).unwrap();
    }

    // Create entry point
    fs::write(
        src_dir.join("main.py"),
        r#"
from level_0 import function_0

def main():
    result = function_0(0)
    print(f"Final result: {result}")

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    assert!(
        output.status.success(),
        "Should handle 50-level deep call chain"
    );
}

/// Test Python with complex cross-module dependencies
#[test]
fn test_python_complex_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create interconnected modules
    fs::write(
        src_dir.join("models.py"),
        r#"
from typing import List, Optional
from database import Database
from validators import validate_email, validate_age

class User:
    def __init__(self, name: str, email: str, age: int):
        self.name = name
        self.email = validate_email(email)
        self.age = validate_age(age)
        self.db = Database()
    
    def save(self):
        self.db.save_user(self)

class Group:
    def __init__(self, name: str):
        self.name = name
        self.users: List[User] = []
        self.db = Database()
    
    def add_user(self, user: User):
        self.users.append(user)
        self.db.save_group(self)
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("database.py"),
        r#"
from typing import TYPE_CHECKING
import json

if TYPE_CHECKING:
    from models import User, Group

class Database:
    def __init__(self):
        self.data = {"users": [], "groups": []}
    
    def save_user(self, user: 'User'):
        self.data["users"].append({"name": user.name, "email": user.email})
    
    def save_group(self, group: 'Group'):
        self.data["groups"].append({
            "name": group.name,
            "users": [u.name for u in group.users]
        })
    
    def export(self):
        from exporters import JSONExporter, XMLExporter
        json_exp = JSONExporter()
        xml_exp = XMLExporter()
        return json_exp.export(self.data), xml_exp.export(self.data)
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("validators.py"),
        r#"
import re
from exceptions import ValidationError

def validate_email(email: str) -> str:
    pattern = r'^[\w\.-]+@[\w\.-]+\.\w+$'
    if not re.match(pattern, email):
        raise ValidationError(f"Invalid email: {email}")
    return email

def validate_age(age: int) -> int:
    if age < 0 or age > 150:
        raise ValidationError(f"Invalid age: {age}")
    return age

def validate_username(username: str) -> str:
    from models import User  # Circular import
    if len(username) < 3:
        raise ValidationError("Username too short")
    return username
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("exceptions.py"),
        r#"
class ValidationError(Exception):
    pass

class DatabaseError(Exception):
    pass

class ExportError(Exception):
    pass
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("exporters.py"),
        r#"
import json
import xml.etree.ElementTree as ET
from exceptions import ExportError

class JSONExporter:
    def export(self, data):
        try:
            return json.dumps(data, indent=2)
        except Exception as e:
            raise ExportError(f"JSON export failed: {e}")

class XMLExporter:
    def export(self, data):
        try:
            root = ET.Element("data")
            # Simplified XML export
            return ET.tostring(root, encoding='unicode')
        except Exception as e:
            raise ExportError(f"XML export failed: {e}")
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("main.py"),
        r#"
from models import User, Group
from database import Database
from validators import validate_username

def main():
    # Create users
    user1 = User("Alice", "alice@example.com", 25)
    user2 = User("Bob", "bob@example.com", 30)
    
    # Create group
    group = Group("Developers")
    group.add_user(user1)
    group.add_user(user2)
    
    # Save to database
    user1.save()
    user2.save()
    
    # Export data
    db = Database()
    json_data, xml_data = db.export()
    
    print("Data exported successfully")

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
    assert!(stdout.contains("main.py"));
    assert!(stdout.contains("models.py"));
    assert!(stdout.contains("database.py"));
    assert!(stdout.contains("validators.py"));
}

/// Test Python with thousands of functions in one file
#[test]
fn test_python_many_functions() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let mut content = String::new();
    content.push_str("# File with many functions\n\n");

    // Create 1000 functions
    let function_count = 1000;
    for i in 0..function_count {
        content.push_str(&format!(
            r#"
def func_{i}(x):
    """Function number {i}"""
    return x + {i}

"#
        ));
    }

    // Add a main function that calls some of them
    content.push_str("def main():\n    total = 0\n");
    for i in 0..20 {
        content.push_str(&format!("    total += func_{i}({i})\n"));
    }
    content.push_str("    return total\n\n");
    content.push_str("if __name__ == '__main__':\n    print(main())\n");

    fs::write(src_dir.join("many_functions.py"), content).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success(), "Should handle 1000 functions");
}

/// Test Python with very large classes
#[test]
fn test_python_large_classes() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let mut content = String::new();
    content.push_str("# File with very large classes\n\n");

    // Create a class with many methods
    content.push_str("class MegaClass:\n");
    content.push_str("    \"\"\"A class with many methods.\"\"\"\n\n");

    // Add 500 methods
    for i in 0..500 {
        content.push_str(&format!(
            r#"    def method_{i}(self):
        """Method number {i}"""
        return {i}

"#
        ));
    }

    // Add 500 class methods
    for i in 0..500 {
        content.push_str(&format!(
            r#"    @classmethod
    def class_method_{i}(cls):
        """Class method number {i}"""
        return {i}

"#
        ));
    }

    // Create instance and use some methods
    content.push_str("\n\ndef test_mega_class():\n");
    content.push_str("    obj = MegaClass()\n");
    for i in 0..10 {
        content.push_str(&format!("    obj.method_{i}()\n"));
        content.push_str(&format!("    MegaClass.class_method_{i}()\n"));
    }

    fs::write(src_dir.join("large_classes.py"), content).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success(), "Should handle large classes");
}

/// Test Python with extremely deep nesting
#[test]
fn test_python_deep_nesting() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");

    // Create deeply nested package structure
    let mut current_dir = src_dir.clone();
    let depth = 20;

    for i in 0..depth {
        current_dir = current_dir.join(format!("level{i}"));
        fs::create_dir_all(&current_dir).unwrap();

        // Add __init__.py
        fs::write(
            current_dir.join("__init__.py"),
            format!(
                r#"
"""Level {i} package"""
from .module import func_{i}
"#
            ),
        )
        .unwrap();

        // Add module.py
        fs::write(
            current_dir.join("module.py"),
            format!(
                r#"
def func_{i}():
    """Function at level {i}"""
    return {i}
"#
            ),
        )
        .unwrap();
    }

    // Create main file that imports from deep nesting
    fs::write(
        src_dir.join("main.py"),
        r#"
# Import from deeply nested module
from level0.level1.level2.level3.module import func_3

def main():
    result = func_3()
    print(f"Result from deep import: {result}")

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success(), "Should handle deep nesting");
}

/// Test Python with mixed file types
#[test]
fn test_python_mixed_files() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Regular Python file
    fs::write(
        src_dir.join("regular.py"),
        r#"
def regular_function():
    return "regular"
"#,
    )
    .unwrap();

    // Python file with .pyw extension (Windows GUI scripts)
    fs::write(
        src_dir.join("gui_script.pyw"),
        r#"
import tkinter as tk

def create_window():
    root = tk.Tk()
    return root
"#,
    )
    .unwrap();

    // Cython-like file (though not compiled)
    fs::write(
        src_dir.join("cython_module.pyx"),
        r#"
# This would be Cython code
def fast_function(int x):
    cdef int result = x * 2
    return result
"#,
    )
    .unwrap();

    // Python stub file
    fs::write(
        src_dir.join("stubs.pyi"),
        r#"
# Type stub file
def typed_function(x: int) -> str: ...

class TypedClass:
    def method(self) -> None: ...
"#,
    )
    .unwrap();

    // Setup.py
    fs::write(
        src_dir.join("setup.py"),
        r#"
from setuptools import setup, find_packages

setup(
    name="test-package",
    version="0.1.0",
    packages=find_packages(),
)
"#,
    )
    .unwrap();

    // __main__.py for package execution
    fs::write(
        src_dir.join("__main__.py"),
        r#"
from regular import regular_function

if __name__ == "__main__":
    print(regular_function())
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success());
}

/// Test Python with various string types and formats
#[test]
fn test_python_string_complexity() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("strings.py"),
        r#"
# Various string formats and edge cases

# Triple quoted strings with quotes inside
sql_query = """
SELECT * FROM users
WHERE name = 'John' AND status = "active"
AND description = '''multiple quotes'''
"""

# Raw strings with backslashes
regex_pattern = r"C:\Users\(?P<username>\w+)\Documents\.*\.txt"
raw_multiline = r"""
Line 1\n is literal
Line 2\t has literal tabs
"""

# F-strings with complex expressions
name = "Alice"
age = 30
complex_fstring = f"""
User: {name.upper()}
Age in months: {age * 12}
Is adult: {age >= 18}
Nested: {f"Hello {name}"}
Expression: {[x**2 for x in range(5)]}
"""

# Unicode strings
unicode_art = """
╔═══════════════╗
║   Unicode Box ║
╚═══════════════╝
"""

# String with embedded code-like content
code_in_string = '''
def fake_function():
    # This looks like code but it's a string
    import os
    return "not real code"
'''

# Docstring that looks like it has imports
def tricky_function():
    """
    This function does something.
    
    Example:
        >>> import sys
        >>> from collections import defaultdict
        >>> tricky_function()
    
    Note: The above imports are in a docstring, not real imports.
    """
    pass

# Byte strings
byte_data = b"Binary \x00\x01\x02 data"
byte_multiline = b"""
Multi-line
byte string
"""

# Format strings
old_style = "Hello %s, you are %d years old" % (name, age)
new_style = "Hello {}, you are {} years old".format(name, age)
template = "{name} is {age} years old".format(name=name, age=age)
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success(), "Should handle complex strings");
}

/// Test Python with concurrent/parallel patterns
#[test]
fn test_python_concurrency_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("concurrency.py"),
        r#"
import asyncio
import threading
import multiprocessing
from concurrent.futures import ThreadPoolExecutor, ProcessPoolExecutor
from typing import List, Callable
import queue

# Threading example
class ThreadedWorker(threading.Thread):
    def __init__(self, task_queue: queue.Queue):
        super().__init__()
        self.task_queue = task_queue
        self.daemon = True
    
    def run(self):
        while True:
            task = self.task_queue.get()
            if task is None:
                break
            task()
            self.task_queue.task_done()

# Multiprocessing example
def process_worker(data):
    return sum(data)

def parallel_sum(data_chunks: List[List[int]]) -> int:
    with ProcessPoolExecutor() as executor:
        results = executor.map(process_worker, data_chunks)
        return sum(results)

# Asyncio patterns
async def fetch_data(session, url):
    # Simulated async fetch
    await asyncio.sleep(0.1)
    return f"Data from {url}"

async def fetch_many(urls: List[str]):
    tasks = []
    for url in urls:
        task = asyncio.create_task(fetch_data(None, url))
        tasks.append(task)
    
    results = await asyncio.gather(*tasks)
    return results

# Thread pool pattern
def thread_pool_example(functions: List[Callable]):
    with ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(func) for func in functions]
        results = [f.result() for f in futures]
        return results

# Lock and synchronization
class SharedCounter:
    def __init__(self):
        self._value = 0
        self._lock = threading.Lock()
    
    def increment(self):
        with self._lock:
            self._value += 1
    
    @property
    def value(self):
        with self._lock:
            return self._value

# Asyncio with threading
def run_async_in_thread(coro):
    def thread_target():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            return loop.run_until_complete(coro)
        finally:
            loop.close()
    
    thread = threading.Thread(target=thread_target)
    thread.start()
    thread.join()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    assert!(output.status.success());
}
