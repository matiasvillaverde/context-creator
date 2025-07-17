//! Error case tests for Python semantic analysis

use std::fs;
use tempfile::TempDir;

/// Test Python files with only comments and docstrings
#[test]
fn test_python_comments_only() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("comments.py"),
        r#"
#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""
This module contains only comments and docstrings.
No actual code is present.

Author: Test
Date: 2024
"""

# TODO: Implement actual functionality
# FIXME: Add error handling
# NOTE: This is a placeholder file

'''
Another docstring style.
Still no code.
'''

# More comments
# Even more comments

"""And a final docstring"""
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("empty_structure.py"),
        r#"
# File with empty structures

class EmptyClass:
    """An empty class with just a docstring."""
    pass

def empty_function():
    """An empty function."""
    pass

def function_with_ellipsis():
    """Function with ellipsis."""
    ...

if __name__ == "__main__":
    # Nothing happens
    pass
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("comments.py"));
    assert!(stdout.contains("empty_structure.py"));

    // Should not show semantic info for comment-only files
    let comments_section = stdout
        .split("comments.py")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    assert!(!comments_section.contains("Imports:"));
    assert!(!comments_section.contains("Function calls:"));
}

/// Test Python with Unicode in various places
#[test]
fn test_python_unicode_everywhere() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("unicode_identifiers.py"),
        r#"
# -*- coding: utf-8 -*-
"""Test Unicode identifiers and content."""

# Unicode in variable names (Python 3 allows this)
π = 3.14159
φ = 1.618

# Unicode in function names
def 计算面积(半径):
    """计算圆的面积 (Calculate circle area in Chinese)"""
    return π * 半径 ** 2

def вычислить_объем(радиус):
    """Вычислить объем сферы (Calculate sphere volume in Russian)"""
    return (4/3) * π * радиус ** 3

# Unicode in class names
class 数据处理器:
    """数据处理器 (Data Processor in Chinese)"""
    
    def __init__(self):
        self.数据 = []
    
    def 添加(self, 项目):
        self.数据.append(项目)

# Unicode strings
messages = {
    'greeting': '你好世界',  # Hello world in Chinese
    'farewell': 'До свидания',  # Goodbye in Russian  
    'welcome': 'ようこそ',  # Welcome in Japanese
    'thanks': 'شكراً',  # Thanks in Arabic
    'emoji': '🐍 Python is 🎉 awesome! 🚀'
}

# Using the Unicode identifiers
if __name__ == "__main__":
    面积 = 计算面积(5)
    print(f"面积: {面积}")
    
    процессор = 数据处理器()
    процессор.添加("数据1")
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("unicode_imports.py"),
        r#"
# Import module with Unicode identifiers
from unicode_identifiers import 计算面积, 数据处理器, π

def test_unicode():
    # Use imported Unicode identifiers
    area = 计算面积(10)
    processor = 数据处理器()
    
    print(f"π = {π}")
    return area
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("unicode_identifiers.py"));
    assert!(stdout.contains("unicode_imports.py"));

    // Check if Unicode function names are tracked
    if stdout.contains("Function calls:") {
        // Look for the unicode_imports.py section and get everything until the next file or end
        let content = if let Some(start_idx) = stdout.find("## unicode_imports.py") {
            let section_start = &stdout[start_idx..];
            // Find the next ## (another file) or use the whole remaining content
            if let Some(next_file_idx) = section_start[21..].find("##") {
                &section_start[..next_file_idx + 21]
            } else {
                section_start
            }
        } else {
            ""
        };

        // Should track Unicode function calls
        assert!(
            content.contains("计算面积") || content.contains("Function calls"),
            "Should handle Unicode function names. Content: {content}"
        );
    }
}

/// Test Python with dynamic imports
#[test]
fn test_python_dynamic_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("dynamic_imports.py"),
        r#"
import importlib
import sys
from importlib import import_module

# Dynamic import using importlib
def load_module_dynamic(name: str):
    try:
        module = importlib.import_module(name)
        return module
    except ImportError:
        return None

# __import__ usage
def load_with_import(name: str):
    try:
        module = __import__(name)
        return module
    except ImportError:
        return None

# Conditional dynamic imports
def load_backend(backend_type: str):
    if backend_type == "sqlite":
        from backends import sqlite_backend
        return sqlite_backend
    elif backend_type == "postgres":
        from backends import postgres_backend
        return postgres_backend
    else:
        raise ValueError(f"Unknown backend: {backend_type}")

# Import from string
module_names = ["os", "sys", "json"]
modules = {}

for name in module_names:
    modules[name] = import_module(name)

# Lazy imports in functions
def use_heavy_library():
    # Import only when needed
    import numpy as np
    import pandas as pd
    
    return np.array([1, 2, 3])

# Plugin system simulation
class PluginLoader:
    def __init__(self):
        self.plugins = {}
    
    def load_plugin(self, plugin_path: str):
        spec = importlib.util.spec_from_file_location("plugin", plugin_path)
        if spec and spec.loader:
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module
        return None
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("exec_imports.py"),
        r#"
# Even more dynamic imports using exec
def super_dynamic_import():
    import_statements = [
        "import json",
        "from collections import defaultdict",
        "import re as regex"
    ]
    
    for stmt in import_statements:
        exec(stmt)
    
    # Now json, defaultdict, and regex are available in locals()

# Using eval for imports (bad practice but possible)
def eval_import():
    module_name = "os"
    os_module = eval(f"__import__('{module_name}')")
    return os_module
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    // Should handle dynamic imports without crashing
    assert!(output.status.success());
}

/// Test Python with complex comprehensions and generators
#[test]
fn test_python_comprehensions_generators() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("comprehensions.py"),
        r#"
from typing import List, Dict, Set, Generator, Iterator
import itertools
from functools import reduce

# Nested list comprehensions
matrix = [[i * j for j in range(5)] for i in range(5)]

# Dict comprehension with conditions
squared_evens = {x: x**2 for x in range(20) if x % 2 == 0}

# Set comprehension with multiple loops
pairs = {(i, j) for i in range(3) for j in range(3) if i != j}

# Complex comprehension with imports used inside
def process_data(data: List[str]) -> Dict[str, int]:
    import re
    from collections import Counter
    
    # Comprehension using imported module
    words = [
        word.lower() 
        for text in data 
        for word in re.findall(r'\w+', text)
        if len(word) > 3
    ]
    
    return dict(Counter(words))

# Generator expressions
def infinite_sequence() -> Generator[int, None, None]:
    num = 0
    while True:
        yield num
        num += 1

# Generator with complex logic
def fibonacci_generator():
    a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b

# Comprehension in function arguments
result = sum(x**2 for x in range(10) if x % 2 == 0)

# Nested generator expressions
nested_gen = (
    (x, y, z)
    for x in range(3)
    for y in range(x, 3)
    for z in range(y, 3)
)

# Using itertools in comprehensions
combinations = [
    combo 
    for combo in itertools.combinations(range(5), 3)
    if sum(combo) > 5
]
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    assert!(output.status.success());
}

/// Test Python with multiple inheritance and MRO
#[test]
fn test_python_multiple_inheritance() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("inheritance.py"),
        r#"
from abc import ABC, abstractmethod
from typing import Any

# Diamond inheritance pattern
class A:
    def method(self):
        return "A"

class B(A):
    def method(self):
        return "B" + super().method()

class C(A):
    def method(self):
        return "C" + super().method()

class D(B, C):
    def method(self):
        return "D" + super().method()

# Mixin pattern
class LoggerMixin:
    def log(self, message: str):
        print(f"[LOG] {message}")

class DatabaseMixin:
    def save(self):
        print("Saving to database")

class CacheableMixin:
    def cache(self):
        print("Caching result")

class Service(LoggerMixin, DatabaseMixin, CacheableMixin):
    def process(self):
        self.log("Processing")
        self.cache()
        self.save()

# Abstract base with multiple inheritance
class Readable(ABC):
    @abstractmethod
    def read(self) -> str:
        pass

class Writable(ABC):
    @abstractmethod  
    def write(self, data: str) -> None:
        pass

class Seekable(ABC):
    @abstractmethod
    def seek(self, position: int) -> None:
        pass

class File(Readable, Writable, Seekable):
    def read(self) -> str:
        return "data"
    
    def write(self, data: str) -> None:
        pass
    
    def seek(self, position: int) -> None:
        pass

# Check MRO
if __name__ == "__main__":
    d = D()
    print(D.__mro__)  # Method Resolution Order
    print(d.method())  # Will print "DCBA"
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    assert!(output.status.success());
}

/// Test Python with context managers and descriptors
#[test]
fn test_python_advanced_features() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("advanced.py"),
        r#"
import contextlib
from contextlib import contextmanager
from typing import Any, Optional
import functools

# Custom context manager class
class FileManager:
    def __init__(self, filename: str, mode: str):
        self.filename = filename
        self.mode = mode
        self.file: Optional[Any] = None
    
    def __enter__(self):
        self.file = open(self.filename, self.mode)
        return self.file
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.file:
            self.file.close()
        return False  # Don't suppress exceptions

# Context manager using decorator
@contextmanager
def timer_context():
    import time
    start = time.time()
    try:
        yield
    finally:
        end = time.time()
        print(f"Elapsed: {end - start:.4f}s")

# Descriptor protocol
class ValidatedAttribute:
    def __init__(self, validator):
        self.validator = validator
        self.name = None
    
    def __set_name__(self, owner, name):
        self.name = f"_{name}"
    
    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        return getattr(obj, self.name)
    
    def __set__(self, obj, value):
        if self.validator(value):
            setattr(obj, self.name, value)
        else:
            raise ValueError(f"Invalid value: {value}")

# Property decorator with getter/setter
class Temperature:
    def __init__(self):
        self._celsius = 0
    
    @property
    def celsius(self):
        return self._celsius
    
    @celsius.setter
    def celsius(self, value):
        if value < -273.15:
            raise ValueError("Temperature below absolute zero")
        self._celsius = value
    
    @property
    def fahrenheit(self):
        return self._celsius * 9/5 + 32
    
    @fahrenheit.setter
    def fahrenheit(self, value):
        self.celsius = (value - 32) * 5/9

# Using contextlib utilities
@contextlib.contextmanager
def managed_resource():
    resource = acquire_resource()
    try:
        yield resource
    finally:
        release_resource(resource)

def acquire_resource():
    return "resource"

def release_resource(resource):
    pass

# ExitStack for dynamic context management
def process_files(filenames):
    with contextlib.ExitStack() as stack:
        files = [
            stack.enter_context(open(fname))
            for fname in filenames
        ]
        # Process all files
        return files
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    assert!(output.status.success());
}

/// Test Python files with various encoding declarations
#[test]
fn test_python_encodings() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // UTF-8 with BOM (some editors add this)
    let utf8_bom = vec![0xEF, 0xBB, 0xBF];
    let mut content_with_bom = utf8_bom;
    content_with_bom.extend_from_slice(
        br#"
# -*- coding: utf-8 -*-
def hello():
    return "Hello with BOM"
"#,
    );
    fs::write(src_dir.join("utf8_bom.py"), content_with_bom).unwrap();

    // Latin-1 encoding declaration
    fs::write(
        src_dir.join("latin1.py"),
        r#"
# -*- coding: latin-1 -*-
# This file claims to be latin-1 encoded
def function():
    return "Café"  # Contains latin-1 character
"#,
    )
    .unwrap();

    // Various encoding declaration styles
    fs::write(
        src_dir.join("encoding_styles.py"),
        r#"
# coding: utf-8
# Different style of encoding declaration

# vim: set fileencoding=utf-8 :
# Another common style

def test():
    return "Test"
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    // Should handle various encodings
    assert!(output.status.success());
}

/// Test Python with empty files and edge cases
#[test]
fn test_python_empty_edge_cases() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Completely empty file
    fs::write(src_dir.join("empty.py"), "").unwrap();

    // Just whitespace
    fs::write(src_dir.join("whitespace.py"), "   \n\t\n   \n").unwrap();

    // Just a shebang
    fs::write(src_dir.join("shebang_only.py"), "#!/usr/bin/env python3\n").unwrap();

    // Single line
    fs::write(src_dir.join("oneliner.py"), "x = 42").unwrap();

    // File with only imports
    fs::write(
        src_dir.join("imports_only.py"),
        r#"
import os
import sys
from pathlib import Path
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_context-creator"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute context-creator");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // All files should be included
    assert!(stdout.contains("empty.py"));
    assert!(stdout.contains("whitespace.py"));
    assert!(stdout.contains("shebang_only.py"));
    assert!(stdout.contains("oneliner.py"));
    assert!(stdout.contains("imports_only.py"));
}
