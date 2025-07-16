//! Edge case tests for Python semantic analysis

use std::fs;
use tempfile::TempDir;

/// Test Python circular imports
#[test]
fn test_python_circular_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create circular import: a.py -> b.py -> c.py -> a.py
    fs::write(
        src_dir.join("a.py"),
        r#"
from b import function_b

def function_a():
    return function_b() + 1
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("b.py"),
        r#"
from c import function_c

def function_b():
    return function_c() + 2
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("c.py"),
        r#"
# Circular import - typically would cause issues at runtime
# from a import function_a

def function_c():
    # Lazy import to avoid circular import error
    from a import function_a
    return 10
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should handle circular imports without crashing
    assert!(output.status.success());

    // Each file should show its imports
    assert!(stdout.contains("a.py"));
    assert!(stdout.contains("b.py"));
    assert!(stdout.contains("c.py"));
}

/// Test Python files with no imports
#[test]
fn test_python_no_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Standalone file with no imports
    fs::write(
        src_dir.join("standalone.py"),
        r#"
# No imports, just pure Python
CONSTANT = 42

def pure_function(x, y):
    """A function with no dependencies."""
    return x + y

class StandaloneClass:
    """A class with no external dependencies."""
    
    def __init__(self):
        self.value = CONSTANT
    
    def method(self):
        return self.value * 2

# Script execution
if __name__ == "__main__":
    obj = StandaloneClass()
    result = pure_function(10, obj.method())
    print(f"Result: {result}")
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    let standalone_section = stdout
        .split("## standalone.py")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    // Should not show import sections for files without imports
    assert!(!standalone_section.contains("Imports:"));
    assert!(!standalone_section.contains("Imported by:"));
}

/// Test Python with many import styles
#[test]
fn test_python_import_variations() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("imports.py"),
        r#"
# Standard library imports
import os
import sys
import json
from pathlib import Path
from collections import defaultdict, Counter
from typing import (
    List, Dict, Optional, Union,
    Tuple, Set, Any, TypeVar,
    Generic, Protocol, Final
)

# Try different import styles
import math as mathematics
from datetime import datetime as dt
from itertools import *  # Star import
from functools import (
    wraps,
    lru_cache,
    partial as make_partial
)

# Conditional imports
try:
    import numpy as np
except ImportError:
    np = None

if sys.version_info >= (3, 9):
    from typing import Annotated
else:
    try:
        from typing_extensions import Annotated
    except ImportError:
        Annotated = None

# Type checking imports
from typing import TYPE_CHECKING
if TYPE_CHECKING:
    from some_heavy_module import HeavyClass

# Local imports with different styles
from . import sibling_module
from .. import parent_module
from ..utils import helper
from ...root import config

# Function using imports
def process_data(data: List[Dict[str, Any]]) -> Counter:
    path = Path("data.json")
    counts = Counter()
    
    if np is not None:
        # Use numpy if available
        pass
    
    return counts
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should handle various import styles
    assert!(output.status.success());
    assert!(stdout.contains("imports.py"));
}

/// Test Python namespace packages
#[test]
fn test_python_namespace_packages() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create namespace package structure (no __init__.py in namespace)
    let ns_dir = root.join("namespace_pkg");
    let sub1_dir = ns_dir.join("sub1");
    let sub2_dir = ns_dir.join("sub2");

    fs::create_dir_all(&sub1_dir).unwrap();
    fs::create_dir_all(&sub2_dir).unwrap();

    // sub1 has __init__.py
    fs::write(sub1_dir.join("__init__.py"), "").unwrap();
    fs::write(
        sub1_dir.join("module1.py"),
        r#"
def function1():
    return "from sub1"
"#,
    )
    .unwrap();

    // sub2 has __init__.py
    fs::write(sub2_dir.join("__init__.py"), "").unwrap();
    fs::write(
        sub2_dir.join("module2.py"),
        r#"
def function2():
    return "from sub2"
"#,
    )
    .unwrap();

    // User script
    fs::write(
        root.join("use_namespace.py"),
        r#"
# Import from namespace package
from namespace_pkg.sub1 import module1
from namespace_pkg.sub2 import module2

def main():
    print(module1.function1())
    print(module2.function2())
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(root)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    // Should handle namespace packages
    assert!(output.status.success());
}

/// Test Python with decorators and metaclasses
#[test]
fn test_python_decorators_metaclasses() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("decorators.py"),
        r#"
from functools import wraps
from typing import Callable, TypeVar, Any
import time

T = TypeVar('T')

def timer(func: Callable[..., T]) -> Callable[..., T]:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> T:
        start = time.time()
        result = func(*args, **kwargs)
        print(f"{func.__name__} took {time.time() - start:.4f}s")
        return result
    return wrapper

def memoize(func: Callable[..., T]) -> Callable[..., T]:
    cache: Dict[Any, T] = {}
    
    @wraps(func)
    def wrapper(*args: Any) -> T:
        if args in cache:
            return cache[args]
        result = func(*args)
        cache[args] = result
        return result
    return wrapper

def singleton(cls):
    instances = {}
    @wraps(cls)
    def get_instance(*args, **kwargs):
        if cls not in instances:
            instances[cls] = cls(*args, **kwargs)
        return instances[cls]
    return get_instance
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("metaclasses.py"),
        r#"
from abc import ABCMeta, abstractmethod
from typing import Type, Any

class SingletonMeta(type):
    _instances = {}
    
    def __call__(cls, *args, **kwargs):
        if cls not in cls._instances:
            cls._instances[cls] = super().__call__(*args, **kwargs)
        return cls._instances[cls]

class AutoPropertyMeta(type):
    def __new__(mcs, name: str, bases: tuple, attrs: dict):
        # Auto-generate properties for _private attributes
        for key, value in list(attrs.items()):
            if key.startswith('_') and not key.startswith('__'):
                prop_name = key[1:]  # Remove underscore
                attrs[prop_name] = property(
                    lambda self, k=key: getattr(self, k),
                    lambda self, v, k=key: setattr(self, k, v)
                )
        return super().__new__(mcs, name, bases, attrs)

class AbstractBase(metaclass=ABCMeta):
    @abstractmethod
    def process(self) -> Any:
        pass
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("usage.py"),
        r#"
from decorators import timer, memoize, singleton
from metaclasses import SingletonMeta, AutoPropertyMeta, AbstractBase

@timer
@memoize
def fibonacci(n: int) -> int:
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

@singleton
class Config:
    def __init__(self):
        self.settings = {}

class Database(metaclass=SingletonMeta):
    def __init__(self):
        self.connection = None

class Model(metaclass=AutoPropertyMeta):
    def __init__(self):
        self._id = None
        self._name = None

class ConcreteProcessor(AbstractBase):
    def process(self):
        return "Processing..."

# Usage
if __name__ == "__main__":
    result = fibonacci(30)
    config1 = Config()
    config2 = Config()
    assert config1 is config2
    
    db1 = Database()
    db2 = Database()
    assert db1 is db2
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

    // Check that usage.py imports from both modules
    assert!(stdout.contains("usage.py"));
    if stdout.contains("Imports:") {
        let usage_section = stdout
            .split("usage.py")
            .nth(1)
            .unwrap_or("")
            .split("##")
            .next()
            .unwrap_or("");

        assert!(
            usage_section.contains("decorators") && usage_section.contains("metaclasses"),
            "usage.py should import from both modules"
        );
    }
}

/// Test Python with async/await
#[test]
fn test_python_async_await() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("async_utils.py"),
        r#"
import asyncio
from typing import List, Coroutine, Any

async def fetch_data(url: str) -> str:
    # Simulate async fetch
    await asyncio.sleep(0.1)
    return f"Data from {url}"

async def process_batch(items: List[str]) -> List[str]:
    tasks = [fetch_data(item) for item in items]
    results = await asyncio.gather(*tasks)
    return results

class AsyncManager:
    def __init__(self):
        self.queue = asyncio.Queue()
    
    async def add_task(self, task: Any) -> None:
        await self.queue.put(task)
    
    async def process_tasks(self) -> None:
        while not self.queue.empty():
            task = await self.queue.get()
            # Process task
            await asyncio.sleep(0.01)

async def async_context_manager():
    class AsyncResource:
        async def __aenter__(self):
            await asyncio.sleep(0.01)
            return self
        
        async def __aexit__(self, exc_type, exc_val, exc_tb):
            await asyncio.sleep(0.01)
    
    async with AsyncResource() as resource:
        return resource
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("async_main.py"),
        r#"
import asyncio
from async_utils import fetch_data, process_batch, AsyncManager

async def main():
    # Single await
    data = await fetch_data("https://example.com")
    print(data)
    
    # Multiple awaits
    urls = ["url1", "url2", "url3"]
    results = await process_batch(urls)
    
    # Async with manager
    manager = AsyncManager()
    await manager.add_task("task1")
    await manager.process_tasks()
    
    # Async for (simulated)
    async def async_generator():
        for i in range(3):
            await asyncio.sleep(0.01)
            yield i
    
    async for value in async_generator():
        print(value)

if __name__ == "__main__":
    asyncio.run(main())
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Check imports
    assert!(stdout.contains("async_main.py"));
    assert!(stdout.contains("Imports: async_utils"));

    // Check function calls tracking
    if stdout.contains("Function calls:") {
        let main_section = stdout
            .split("async_main.py")
            .nth(1)
            .unwrap_or("")
            .split("##")
            .next()
            .unwrap_or("");

        assert!(
            main_section.contains("fetch_data") || main_section.contains("process_batch"),
            "Should track async function calls"
        );
    }
}

/// Test Python with complex package structure
#[test]
fn test_python_complex_package_structure() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create complex package structure
    let pkg = root.join("myapp");
    let core = pkg.join("core");
    let utils = pkg.join("utils");
    let plugins = pkg.join("plugins");
    let plugin1 = plugins.join("plugin1");
    let tests = pkg.join("tests");

    fs::create_dir_all(&core).unwrap();
    fs::create_dir_all(&utils).unwrap();
    fs::create_dir_all(&plugin1).unwrap();
    fs::create_dir_all(&tests).unwrap();

    // Package root
    fs::write(
        pkg.join("__init__.py"),
        r#"
"""MyApp - A complex application."""
from .core import App
from .version import __version__

__all__ = ['App', '__version__']
"#,
    )
    .unwrap();

    fs::write(pkg.join("version.py"), r#"__version__ = "1.0.0""#).unwrap();

    // Core module
    fs::write(core.join("__init__.py"), r#"from .app import App"#).unwrap();

    fs::write(
        core.join("app.py"),
        r#"
from ..utils import Logger, Config
from ..plugins import load_plugins

class App:
    def __init__(self):
        self.logger = Logger()
        self.config = Config()
        self.plugins = load_plugins()
    
    def run(self):
        self.logger.info("App starting")
        for plugin in self.plugins:
            plugin.execute()
"#,
    )
    .unwrap();

    // Utils module
    fs::write(
        utils.join("__init__.py"),
        r#"
from .logger import Logger
from .config import Config

__all__ = ['Logger', 'Config']
"#,
    )
    .unwrap();

    fs::write(
        utils.join("logger.py"),
        r#"
import logging

class Logger:
    def __init__(self):
        self.logger = logging.getLogger(__name__)
    
    def info(self, msg):
        self.logger.info(msg)
"#,
    )
    .unwrap();

    fs::write(
        utils.join("config.py"),
        r#"
import json
from pathlib import Path

class Config:
    def __init__(self):
        self.data = {}
    
    def load(self, path: Path):
        with open(path) as f:
            self.data = json.load(f)
"#,
    )
    .unwrap();

    // Plugins
    fs::write(
        plugins.join("__init__.py"),
        r#"
from .loader import load_plugins

__all__ = ['load_plugins']
"#,
    )
    .unwrap();

    fs::write(
        plugins.join("loader.py"),
        r#"
from .plugin1 import Plugin1

def load_plugins():
    return [Plugin1()]
"#,
    )
    .unwrap();

    fs::write(
        plugin1.join("__init__.py"),
        r#"from .plugin import Plugin1"#,
    )
    .unwrap();

    fs::write(
        plugin1.join("plugin.py"),
        r#"
from ...utils import Logger

class Plugin1:
    def __init__(self):
        self.logger = Logger()
    
    def execute(self):
        self.logger.info("Plugin1 executing")
"#,
    )
    .unwrap();

    // Main entry point
    fs::write(
        root.join("main.py"),
        r#"
from myapp import App

def main():
    app = App()
    app.run()

if __name__ == "__main__":
    main()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(root)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Verify main imports
    assert!(stdout.contains("main.py"));
    assert!(stdout.contains("Imports: myapp"));

    // Verify internal package imports are tracked
    assert!(stdout.contains("app.py"));
    assert!(stdout.contains("logger.py"));
    assert!(stdout.contains("config.py"));
}

/// Test Python files with syntax errors
#[test]
fn test_python_syntax_errors() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("syntax_error.py"),
        r#"
import os

def broken_function()
    # Missing colon
    print("This won't parse")

class IncompleteClass:
    def __init__(self):
        self.value = 

# Unclosed string
message = "This string never ends...
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("good.py"),
        r#"
# This file is fine
def working_function():
    return 42
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    // Should not crash on syntax errors
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("syntax_error.py"));
    assert!(stdout.contains("good.py"));
}

/// Test Python with very long import lists
#[test]
fn test_python_many_imports() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Create many modules
    let module_count = 50;
    for i in 0..module_count {
        fs::write(
            src_dir.join(format!("module{}.py", i)),
            format!(
                r#"
def function{}():
    return {}
"#,
                i, i
            ),
        )
        .unwrap();
    }

    // Create main file that imports all
    let mut imports = String::new();
    let mut calls = String::new();

    for i in 0..module_count {
        imports.push_str(&format!("from module{} import function{}\n", i, i));
        calls.push_str(&format!("    result += function{}()\n", i));
    }

    fs::write(
        src_dir.join("main.py"),
        format!(
            r#"
{}

def main():
    result = 0
{}
    return result

if __name__ == "__main__":
    print(main())
"#,
            imports, calls
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("main.py"));

    // Should handle many imports
    if stdout.contains("Imports:") {
        assert!(stdout.contains("module0") || stdout.contains("module1"));
    }
}

/// Test Python method calls and attribute access
#[test]
fn test_python_method_calls_and_attributes() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("methods.py"),
        r#"
import json
from pathlib import Path

class DataProcessor:
    def __init__(self):
        self.data = []
        self.path = Path.cwd()
    
    def load_data(self, filename):
        with open(filename) as f:
            self.data = json.load(f)
    
    def process(self):
        # Method calls on self
        self.validate()
        self.transform()
        self.save()
        
        # Chained method calls
        result = self.filter_data().map_values().reduce_results()
        return result
    
    def validate(self):
        return len(self.data) > 0
    
    def transform(self):
        return [item.upper() for item in self.data]
    
    def save(self):
        self.path.write_text(json.dumps(self.data))
    
    def filter_data(self):
        return self
    
    def map_values(self):
        return self
    
    def reduce_results(self):
        return self.data

def use_processor():
    processor = DataProcessor()
    
    # Various method calls
    processor.load_data("data.json")
    processor.process()
    
    # Attribute access
    print(processor.data)
    print(processor.path.name)
    
    # Method calls on imported objects
    path = Path("test.txt")
    path.exists()
    path.is_file()
    path.read_text()
    
    # JSON method calls
    data = {"key": "value"}
    json_str = json.dumps(data)
    parsed = json.loads(json_str)
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Should track method calls
    if stdout.contains("Function calls:") {
        let content = stdout
            .split("methods.py")
            .nth(1)
            .unwrap_or("")
            .split("##")
            .next()
            .unwrap_or("");

        // Should track various types of method calls
        assert!(
            content.contains("load_data")
                || content.contains("process")
                || content.contains("json.load")
                || content.contains("json.dumps")
                || content.contains("Path.cwd"),
            "Should track method calls including json.load, json.dumps, etc."
        );
    }
}

/// Test Python list/dict comprehensions with function calls
#[test]
fn test_python_comprehension_function_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("comprehensions.py"),
        r#"
import math
from utils import process_item, validate_item, transform_data

def complex_comprehensions():
    # List comprehension with function calls
    data = [1, 2, 3, 4, 5]
    
    # Functions called inside comprehensions
    processed = [process_item(x) for x in data if validate_item(x)]
    
    # Math functions in comprehensions  
    squares = [math.sqrt(x) for x in data]
    powers = [math.pow(x, 2) for x in data]
    
    # Dict comprehension with function calls
    data_dict = {
        str(x): transform_data(x) 
        for x in data 
        if validate_item(x)
    }
    
    # Set comprehension with multiple function calls
    unique_processed = {
        process_item(transform_data(x))
        for x in data
    }
    
    # Nested comprehensions with function calls
    matrix = [
        [math.sin(i * j) for j in range(5)]
        for i in range(5)
    ]
    
    # Generator expression with function calls
    gen = (process_item(x) for x in data if x > 2)
    
    # Using map, filter with functions
    mapped = list(map(process_item, data))
    filtered = list(filter(validate_item, data))
    
    return processed, data_dict, unique_processed

# Function calls in lambda expressions
operations = {
    'process': lambda x: process_item(x),
    'validate': lambda x: validate_item(x),
    'transform': lambda x: transform_data(x),
    'complex': lambda x: process_item(transform_data(x))
}
"#,
    )
    .unwrap();

    fs::write(
        src_dir.join("utils.py"),
        r#"
def process_item(item):
    return item * 2

def validate_item(item):
    return item > 0

def transform_data(data):
    return data ** 2
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(&src_dir)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Should track imports
    assert!(stdout.contains("comprehensions.py"));
    assert!(stdout.contains("Imports: utils"));

    // Should track function calls inside comprehensions
    if stdout.contains("Function calls:") {
        let content = stdout
            .split("comprehensions.py")
            .nth(1)
            .unwrap_or("")
            .split("##")
            .next()
            .unwrap_or("");

        assert!(
            content.contains("process_item")
                || content.contains("validate_item")
                || content.contains("transform_data")
                || content.contains("math.sqrt")
                || content.contains("math.pow"),
            "Should track function calls inside comprehensions and lambdas"
        );
    }
}

/// Test Python imports from __init__.py and package structure
#[test]
fn test_python_init_file_imports() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create package with __init__.py imports
    let pkg = root.join("mypackage");
    let subpkg = pkg.join("submodule");
    fs::create_dir_all(&subpkg).unwrap();

    // Root package __init__.py with imports and re-exports
    fs::write(
        pkg.join("__init__.py"),
        r#"
"""Main package with various import patterns."""

# Import from submodules
from .core import Engine, Config
from .utils import helper, utility_function
from .constants import DEFAULT_TIMEOUT, MAX_RETRIES

# Import and rename
from .processor import DataProcessor as Processor

# Import all from a module
from .types import *

# Conditional imports
import sys
if sys.version_info >= (3, 8):
    from .modern import ModernFeature
else:
    from .legacy import LegacyFeature as ModernFeature

# Lazy imports in functions
def get_optional_module():
    try:
        from .optional import OptionalFeature
        return OptionalFeature
    except ImportError:
        return None

# Re-export pattern
__all__ = [
    'Engine',
    'Config', 
    'Processor',
    'helper',
    'DEFAULT_TIMEOUT',
    'ModernFeature'
]

# Package-level initialization
_initialized = False

def init_package():
    global _initialized
    if not _initialized:
        from .setup import configure
        configure()
        _initialized = True
"#,
    )
    .unwrap();

    // Create referenced modules
    fs::write(
        pkg.join("core.py"),
        r#"
class Engine:
    pass

class Config:
    pass
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("utils.py"),
        r#"
def helper():
    return "helper"

def utility_function():
    return "utility"
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("constants.py"),
        r#"
DEFAULT_TIMEOUT = 30
MAX_RETRIES = 3
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("processor.py"),
        r#"
class DataProcessor:
    pass
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("types.py"),
        r#"
class TypeA:
    pass

class TypeB:
    pass
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("modern.py"),
        r#"
class ModernFeature:
    pass
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("legacy.py"),
        r#"
class LegacyFeature:
    pass
"#,
    )
    .unwrap();

    fs::write(
        pkg.join("setup.py"),
        r#"
def configure():
    pass
"#,
    )
    .unwrap();

    // User code importing from the package
    fs::write(
        root.join("use_package.py"),
        r#"
# Import from package __init__.py exports
from mypackage import Engine, Config, Processor, helper
from mypackage import DEFAULT_TIMEOUT

# Import the package itself
import mypackage

def main():
    # Use imported items
    engine = Engine()
    config = Config()
    processor = Processor()
    
    result = helper()
    print(f"Timeout: {DEFAULT_TIMEOUT}")
    
    # Initialize package
    mypackage.init_package()
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_code-digest"))
        .arg(root)
        .arg("--trace-imports")
        .arg("--include-callers")
        .output()
        .expect("Failed to execute code-digest");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());

    // Should track imports from __init__.py
    let init_section = stdout
        .split("__init__.py")
        .nth(1)
        .unwrap_or("")
        .split("##")
        .next()
        .unwrap_or("");

    if init_section.contains("Imports:") {
        assert!(
            init_section.contains("core")
                || init_section.contains("utils")
                || init_section.contains("constants"),
            "__init__.py should show imports from submodules"
        );
    }

    // Should track that modules are imported by __init__.py
    assert!(
        stdout.contains("core.py") && stdout.contains("Imported by:"),
        "core.py should show it's imported by __init__.py"
    );
}
