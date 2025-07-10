# Sample Mixed Project

This is a sample project demonstrating a mixed-language codebase for testing `code-digest`.

## Features

- Node.js/Express backend
- Python utility scripts
- JavaScript tests
- Multiple configuration files

## Project Structure

```
sample-mixed-project/
├── src/
│   ├── index.js      # Main application entry
│   └── utils.py      # Python utilities
├── tests/
│   └── test_api.js   # API tests
├── docs/
│   └── README.md     # This file
└── package.json      # Node.js dependencies
```

## Running the Project

```bash
# Install dependencies
npm install

# Start the server
npm start

# Run tests
npm test
```

## API Endpoints

- `GET /` - Welcome message
- `GET /api/users` - List all users

## Python Utilities

The `utils.py` file provides data processing capabilities that can be called from the Node.js application or used standalone.