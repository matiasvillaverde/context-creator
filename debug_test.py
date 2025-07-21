#!/usr/bin/env python3
"""Debug script to test Python import resolution"""

import os
import sys
import tempfile
import subprocess
from pathlib import Path

# Create a test project structure
with tempfile.TemporaryDirectory() as tmpdir:
    print(f"Test directory: {tmpdir}")
    
    # Create directories
    os.makedirs(os.path.join(tmpdir, "src", "models"))
    os.makedirs(os.path.join(tmpdir, "src", "services"))
    
    # Create src/models/user.py
    with open(os.path.join(tmpdir, "src", "models", "user.py"), "w") as f:
        f.write("""
class User:
    def __init__(self, id, name):
        self.id = id
        self.name = name
""")
    
    # Create src/services/user_service.py
    with open(os.path.join(tmpdir, "src", "services", "user_service.py"), "w") as f:
        f.write("""
from src.models.user import User

def get_user(user_id):
    return User(user_id, "Test User")
""")
    
    # Run context-creator with glob pattern and trace imports
    # Get the absolute path to the project root
    project_root = Path(__file__).parent.absolute()
    context_creator_path = project_root / "target" / "debug" / "context-creator"
    
    if not context_creator_path.exists():
        # Try to build it first
        print("Building context-creator...")
        build_result = subprocess.run(["cargo", "build"], cwd=str(project_root), capture_output=True, text=True)
        if build_result.returncode != 0:
            print(f"Build failed: {build_result.stderr}")
            sys.exit(1)
    
    cmd = [
        str(context_creator_path),
        "--include", "src/services/*.py",
        "--trace-imports",
        "--verbose", "2"
    ]
    
    print(f"Running: {' '.join(cmd)}")
    print(f"Working directory: {tmpdir}")
    
    result = subprocess.run(cmd, cwd=tmpdir, capture_output=True, text=True)
    
    print("\n=== STDOUT ===")
    print(result.stdout)
    
    print("\n=== STDERR ===")
    print(result.stderr)
    
    # Check if src/models/user.py is included
    if "src/models/user.py" in result.stdout:
        print("\n✓ SUCCESS: src/models/user.py was included!")
    else:
        print("\n✗ FAIL: src/models/user.py was NOT included!")
        print("\nSearching for any mention of user.py in output...")
        if "user.py" in result.stdout or "user.py" in result.stderr:
            print("Found mention of user.py")