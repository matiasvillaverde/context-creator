# Examples

Real-world examples of using context-creator for various scenarios and project types.

## Quick Start Examples

### Basic Project Analysis

```bash
# Analyze current directory
context-creator

# Analyze specific project
context-creator -d ~/projects/my-app

# Save analysis to file
context-creator -d ~/projects/my-app -o analysis.md

# Get quick overview with token limit
context-creator -d ~/projects/my-app --max-tokens 10000
```

### LLM Integration Examples

```bash
# Ask specific questions about the codebase
context-creator -d ~/projects/web-app "What does this application do?"

context-creator -d ~/projects/api "How is authentication implemented?"

context-creator -d ~/projects/ml-model "Explain the machine learning pipeline"

# Code review and analysis
context-creator -d ~/projects/feature-branch "Review this code for security issues"

context-creator -d ~/projects/legacy-app "Identify technical debt and improvement opportunities"
```

## Project Type Examples

### Rust Projects

```bash
# Rust project with Cargo
cd ~/rust-projects/web-server
context-creator -d . -o rust-analysis.md --max-tokens 50000

# Focus on core functionality
context-creator -d src/ --max-tokens 25000 "Explain the main application architecture"

# Configuration for Rust projects
cat > .context-creator.toml << EOF
[defaults]
max_tokens = 75000
progress = true

ignore = ["target/", "Cargo.lock"]

[[priorities]]
pattern = "src/main.rs"
weight = 200.0

[[priorities]]
pattern = "src/lib.rs"
weight = 180.0

[[priorities]]
pattern = "src/**/*.rs"
weight = 150.0

[[priorities]]
pattern = "Cargo.toml"
weight = 120.0
EOF
```

### JavaScript/Node.js Projects

```bash
# Node.js project analysis
cd ~/js-projects/express-api
context-creator -d . -o nodejs-analysis.md

# Focus on source code, ignore dependencies
context-creator -d src/ --max-tokens 40000

# Configuration for Node.js projects
cat > .context-creator.toml << EOF
[defaults]
max_tokens = 60000
progress = true

ignore = [
    "node_modules/",
    "dist/",
    "build/",
    "coverage/",
    "*.log"
]

[[priorities]]
pattern = "src/index.*"
weight = 200.0

[[priorities]]
pattern = "src/**/*.js"
weight = 150.0

[[priorities]]
pattern = "src/**/*.ts"
weight = 150.0

[[priorities]]
pattern = "package.json"
weight = 120.0

[[priorities]]
pattern = "*.config.js"
weight = 100.0
EOF
```

### Python Projects

```bash
# Python project analysis
cd ~/python-projects/django-app
context-creator -d . -o python-analysis.md

# Django-specific analysis
context-creator -d . "Explain the Django models and views structure"

# Configuration for Python projects
cat > .context-creator.toml << EOF
[defaults]
max_tokens = 55000
progress = true

ignore = [
    "__pycache__/",
    "*.pyc",
    "*.pyo",
    ".venv/",
    "venv/",
    ".pytest_cache/",
    "htmlcov/",
    "dist/",
    "build/",
    "*.egg-info/"
]

[[priorities]]
pattern = "main.py"
weight = 200.0

[[priorities]]
pattern = "app.py"
weight = 200.0

[[priorities]]
pattern = "manage.py"
weight = 180.0

[[priorities]]
pattern = "**/*.py"
weight = 150.0

[[priorities]]
pattern = "requirements.txt"
weight = 120.0

[[priorities]]
pattern = "setup.py"
weight = 120.0

[[priorities]]
pattern = "pyproject.toml"
weight = 120.0
EOF
```

### Go Projects

```bash
# Go project analysis
cd ~/go-projects/api-server
context-creator -d . -o go-analysis.md

# Configuration for Go projects
cat > .context-creator.toml << EOF
[defaults]
max_tokens = 50000
progress = true

ignore = [
    "vendor/",
    "bin/",
    "*.exe",
    "*.test",
    ".DS_Store"
]

[[priorities]]
pattern = "main.go"
weight = 200.0

[[priorities]]
pattern = "cmd/**/*.go"
weight = 180.0

[[priorities]]
pattern = "pkg/**/*.go"
weight = 150.0

[[priorities]]
pattern = "internal/**/*.go"
weight = 140.0

[[priorities]]
pattern = "go.mod"
weight = 120.0

[[priorities]]
pattern = "Makefile"
weight = 100.0
EOF
```

### Java Projects

```bash
# Java/Maven project
cd ~/java-projects/spring-boot-app
context-creator -d . -o java-analysis.md

# Configuration for Java projects
cat > .context-creator.toml << EOF
[defaults]
max_tokens = 65000
progress = true

ignore = [
    "target/",
    "build/",
    "*.class",
    "*.jar",
    "*.war",
    ".gradle/",
    "out/"
]

[[priorities]]
pattern = "src/main/java/**/Application.java"
weight = 200.0

[[priorities]]
pattern = "src/main/java/**/*.java"
weight = 150.0

[[priorities]]
pattern = "pom.xml"
weight = 120.0

[[priorities]]
pattern = "build.gradle"
weight = 120.0

[[priorities]]
pattern = "src/main/resources/**/*"
weight = 100.0
EOF
```

## Use Case Examples

### Code Review and Quality Analysis

```bash
# Comprehensive code review
context-creator -d feature-branch "Perform a comprehensive code review focusing on:"
context-creator -d feature-branch "1. Code quality and best practices"
context-creator -d feature-branch "2. Security vulnerabilities"
context-creator -d feature-branch "3. Performance optimization opportunities"
context-creator -d feature-branch "4. Maintainability and documentation"

# Compare with main branch
git diff main..feature-branch --name-only > changed-files.txt
context-creator -d . --include-from changed-files.txt "Review only the changed files"

# Security-focused review
cat > security-review.toml << EOF
[defaults]
max_tokens = 30000
verbose = true

ignore = ["tests/", "docs/"]

[[priorities]]
pattern = "**/auth*"
weight = 300.0

[[priorities]]
pattern = "**/security*"
weight = 300.0

[[priorities]]
pattern = "**/*login*"
weight = 250.0

[[priorities]]
pattern = "**/*password*"
weight = 250.0

[[priorities]]
pattern = "**/*token*"
weight = 200.0
EOF

context-creator -c security-review.toml -d . "Analyze this code for security vulnerabilities"
```

### Documentation Generation

```bash
# Generate API documentation
context-creator -d src/api/ "Generate comprehensive API documentation with:"
context-creator -d src/api/ "1. Endpoint descriptions"
context-creator -d src/api/ "2. Request/response schemas"
context-creator -d src/api/ "3. Authentication requirements"
context-creator -d src/api/ "4. Usage examples"

# Architecture documentation
context-creator -d . --max-tokens 40000 "Create system architecture documentation"

# Onboarding guide
context-creator -d . "Create a comprehensive onboarding guide for new developers"

# Configuration for documentation focus
cat > docs-config.toml << EOF
[defaults]
max_tokens = 50000

ignore = ["tests/", "node_modules/", "target/"]

[[priorities]]
pattern = "README.*"
weight = 300.0

[[priorities]]
pattern = "docs/**/*"
weight = 250.0

[[priorities]]
pattern = "src/**/*.md"
weight = 200.0

[[priorities]]
pattern = "*.config.*"
weight = 150.0
EOF
```

### Legacy Code Analysis

```bash
# Understand legacy codebase
context-creator -d legacy-system "Help me understand this legacy codebase:"
context-creator -d legacy-system "1. What is the main purpose and functionality?"
context-creator -d legacy-system "2. What are the key components and their relationships?"
context-creator -d legacy-system "3. What technologies and frameworks are used?"
context-creator -d legacy-system "4. What are the main pain points and technical debt?"

# Migration planning
context-creator -d old-app "Create a migration plan from this PHP application to modern Node.js"

# Refactoring opportunities
cat > refactor-analysis.toml << EOF
[defaults]
max_tokens = 45000

[[priorities]]
pattern = "**/*legacy*"
weight = 200.0

[[priorities]]
pattern = "**/*old*"
weight = 180.0

[[priorities]]
pattern = "**/*deprecated*"
weight = 160.0

[[priorities]]
pattern = "**/*.php"
weight = 150.0  # If migrating from PHP
EOF

context-creator -c refactor-analysis.toml -d . "Identify refactoring opportunities"
```

### Performance Analysis

```bash
# Performance-focused analysis
context-creator -d . "Analyze this codebase for performance bottlenecks:"
context-creator -d . "1. Identify CPU-intensive operations"
context-creator -d . "2. Find memory leaks and inefficient memory usage"
context-creator -d . "3. Detect slow database queries"
context-creator -d . "4. Suggest caching opportunities"

# Configuration for performance analysis
cat > performance-config.toml << EOF
[defaults]
max_tokens = 40000

[[priorities]]
pattern = "**/*performance*"
weight = 300.0

[[priorities]]
pattern = "**/*benchmark*"
weight = 250.0

[[priorities]]
pattern = "**/*cache*"
weight = 200.0

[[priorities]]
pattern = "**/*db*"
weight = 200.0

[[priorities]]
pattern = "**/*database*"
weight = 200.0

[[priorities]]
pattern = "**/*query*"
weight = 180.0
EOF
```

### Testing and Quality Assurance

```bash
# Test coverage analysis
context-creator -d tests/ "Analyze test coverage and suggest improvements"

# Testing strategy
context-creator -d . "Suggest a comprehensive testing strategy for this project"

# Quality metrics
cat > quality-config.toml << EOF
[defaults]
max_tokens = 35000

[[priorities]]
pattern = "tests/**/*"
weight = 200.0

[[priorities]]
pattern = "**/*test*"
weight = 180.0

[[priorities]]
pattern = "**/*spec*"
weight = 180.0

[[priorities]]
pattern = "**/test_*"
weight = 160.0

[[priorities]]
pattern = "jest.config.*"
weight = 120.0

[[priorities]]
pattern = "pytest.ini"
weight = 120.0

[[priorities]]
pattern = "Cargo.toml"
weight = 120.0  # For Rust test configuration
EOF

context-creator -c quality-config.toml -d . "Evaluate test quality and coverage"
```

## Workflow Integration Examples

### Git Hooks Integration

```bash
# Pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
# Generate code analysis before commit

echo "Generating code analysis..."
context-creator -d . -o .git/commit-analysis.md --max-tokens 20000 --quiet

if [ $? -eq 0 ]; then
    echo "Code analysis generated successfully"
else
    echo "Failed to generate code analysis"
    exit 1
fi
EOF

chmod +x .git/hooks/pre-commit
```

### CI/CD Pipeline Integration

```yaml
# .github/workflows/code-analysis.yml
name: Code Analysis

on:
  pull_request:
    branches: [main]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install context-creator
        run: cargo install context-creator
      
      - name: Analyze changed files
        run: |
          # Get changed files
          git diff --name-only origin/main..HEAD > changed-files.txt
          
          # Generate analysis
          context-creator -d . --include-from changed-files.txt \
            --max-tokens 30000 -o pr-analysis.md
      
      - name: Comment on PR
        uses: actions/github-script@v6
        with:
          script: |
            const fs = require('fs');
            const analysis = fs.readFileSync('pr-analysis.md', 'utf8');
            
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: `## Code Analysis\n\n${analysis}`
            });
```

### VS Code Integration

```json
// .vscode/tasks.json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Generate Code Analysis",
            "type": "shell",
            "command": "context-creator",
            "args": [
                "-d", "${workspaceFolder}",
                "-o", "${workspaceFolder}/docs/analysis.md",
                "--max-tokens", "50000",
                "--progress"
            ],
            "group": "build",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "shared"
            },
            "problemMatcher": []
        },
        {
            "label": "Quick Code Review",
            "type": "shell",
            "command": "context-creator",
            "args": [
                "-d", "${workspaceFolder}",
                "--max-tokens", "20000",
                "Review this code for potential issues"
            ],
            "group": "build"
        }
    ]
}
```

## Advanced Configuration Examples

### Multi-Project Workspace

```toml
# ~/.config/context-creator/config.toml
[defaults]
max_tokens = 50000
progress = true

# Project-specific configurations
[projects."/workspace/frontend"]
max_tokens = 40000
tool = "gemini"
ignore = ["node_modules/", "dist/"]

[projects."/workspace/backend"]
max_tokens = 60000
tool = "codex"
ignore = ["target/", "logs/"]

[projects."/workspace/mobile"]
max_tokens = 35000
ignore = ["build/", "*.xcworkspace"]

[[priorities]]
pattern = "src/main.*"
weight = 200.0

[[priorities]]
pattern = "src/lib.*"
weight = 180.0
```

### Template Configurations

```toml
# rust-template.toml
[defaults]
max_tokens = 75000
progress = true
verbose = false

ignore = [
    "target/",
    "Cargo.lock",
    "*.rlib",
    "*.rmeta"
]

[[priorities]]
pattern = "src/main.rs"
weight = 200.0

[[priorities]]
pattern = "src/lib.rs"
weight = 180.0

[[priorities]]
pattern = "src/**/*.rs"
weight = 150.0

[[priorities]]
pattern = "Cargo.toml"
weight = 120.0

[[priorities]]
pattern = "README.*"
weight = 100.0

# Use with: context-creator -c rust-template.toml -d project/
```

```toml
# web-template.toml
[defaults]
max_tokens = 60000
progress = true

ignore = [
    "node_modules/",
    "dist/",
    "build/",
    "coverage/",
    ".next/",
    ".nuxt/",
    "*.bundle.js",
    "*.chunk.js"
]

[[priorities]]
pattern = "src/App.*"
weight = 200.0

[[priorities]]
pattern = "src/index.*"
weight = 190.0

[[priorities]]
pattern = "src/main.*"
weight = 190.0

[[priorities]]
pattern = "src/**/*.{js,ts,jsx,tsx,vue}"
weight = 150.0

[[priorities]]
pattern = "package.json"
weight = 120.0

[[priorities]]
pattern = "*.config.{js,ts}"
weight = 110.0
```

## Scripting Examples

### Batch Processing Script

```bash
#!/bin/bash
# analyze-projects.sh - Batch analyze multiple projects

PROJECTS=(
    "/workspace/project1"
    "/workspace/project2"
    "/workspace/project3"
)

OUTPUT_DIR="./analyses"
mkdir -p "$OUTPUT_DIR"

for project in "${PROJECTS[@]}"; do
    project_name=$(basename "$project")
    echo "Analyzing $project_name..."
    
    context-creator -d "$project" \
        -o "$OUTPUT_DIR/${project_name}-analysis.md" \
        --max-tokens 50000 \
        --progress
    
    echo "Analysis saved to $OUTPUT_DIR/${project_name}-analysis.md"
done

echo "All analyses complete!"
```

### Interactive Analysis Script

```bash
#!/bin/bash
# interactive-analysis.sh - Interactive code analysis

echo "Code context Interactive Analysis"
echo "================================"

read -p "Enter project directory: " PROJECT_DIR
read -p "Enter max tokens (default 50000): " MAX_TOKENS
MAX_TOKENS=${MAX_TOKENS:-50000}

read -p "Enter analysis question: " QUESTION

echo "Analyzing project..."
context-creator -d "$PROJECT_DIR" \
    --max-tokens "$MAX_TOKENS" \
    --verbose \
    "$QUESTION"
```

### Monitoring Script

```bash
#!/bin/bash
# monitor-codebase.sh - Monitor codebase changes

WATCH_DIR="$1"
OUTPUT_FILE="codebase-analysis.md"

if [ -z "$WATCH_DIR" ]; then
    echo "Usage: $0 <directory-to-watch>"
    exit 1
fi

echo "Monitoring $WATCH_DIR for changes..."

# Generate initial analysis
context-creator -d "$WATCH_DIR" -o "$OUTPUT_FILE" --max-tokens 40000

# Watch for changes
fswatch -o "$WATCH_DIR" | while read change; do
    echo "Changes detected, regenerating analysis..."
    context-creator -d "$WATCH_DIR" -o "$OUTPUT_FILE" --max-tokens 40000 --quiet
    echo "Analysis updated at $(date)"
done
```

## Next Steps

- Read the [Configuration Reference](configuration.md) for detailed options
- Check [Usage Guide](usage.md) for more command examples
- See [API Reference](api.md) for programmatic usage
- Visit [Troubleshooting](troubleshooting.md) for common issues