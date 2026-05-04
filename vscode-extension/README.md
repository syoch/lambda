# Lambda Language Support

VSCode extension for Lambda Calculus script (.lambda) files with LSP (Language Server Protocol) integration.

## Features

- **Syntax Highlighting**: Colorized syntax for Lambda scripts
- **Language Support**: Full support for `.lambda` files
- **LSP Integration**: Language Server Protocol for:
  - Completion suggestions for `reduce`, `reduce_steps`, `assert`, `search`, etc.
  - Inlay Hints for `reduce` and `assert` commands
  - Hover information
- **Commands**:
  - `Lambda: Reduce Expression` - Reduce the current expression to normal form
  - `Lambda: Show Inlay Hints` - Toggle Inlay Hints display

## Installation

### Prerequisites

- **Lambda CLI**: Install from the main repository
  ```bash
  cd /path/to/lambda
  cargo install --path .
  ```
  Ensure `lambda` is in your PATH or note its full path

### Quick Install (VSIX)

1. **Get the VSIX file**:
   - Download from releases, OR
   - Build it yourself (see below)

2. **Install in VSCode**:
   - Press `Ctrl+Shift+X` (Extensions)
   - Click `...` menu → "Install from VSIX"
   - Select the `.vsix` file
   - Reload VSCode

### Build from Source

#### Using Nix (Recommended)
```bash
cd vscode-extension
nix run .#build-vsix
```

#### Using npm
```bash
cd vscode-extension
npm install
npm run compile
npm run package
```

#### Using build script
```bash
cd vscode-extension
bash ./build.sh
```

## Configuration

After installation, configure the Lambda CLI path:

### Method 1: Settings UI
1. Open VSCode Settings (`Ctrl+,`)
2. Search: "lambda.lsp.serverPath"
3. Enter your lambda CLI path (default: `lambda`)

### Method 2: Command Palette
1. Press `Ctrl+Shift+P`
2. Type: "Lambda: Configure LSP Server"
3. Enter the path

### Method 3: settings.json
```json
{
  "lambda.lsp.enable": true,
  "lambda.lsp.serverPath": "lambda",
  "lambda.lsp.serverArgs": ["lsp"],
  "lambda.inlayHints.enable": true,
  "lambda.completion.enable": true,
  "lambda.hover.enable": true
}
```

## Usage

### Basic Script

Create a `.lambda` file:

```lambda
from "basics.lambda" import I, K, S

# Test reduction
reduce I

reduce K I

# Test assertion
assert: I x == (\y. y) x
```

### Commands Reference

```lambda
# Reduce to normal form
reduce <expression>

# Show all reduction steps
reduce_steps[(<max_steps>)] <expression>

# Test equality
assert[(<steps>)]: <left> == <right>

# Search for combinations
search[(<n>, <max_steps>)] <base> -> <target>

# Include file
include "path/to/file.lambda" [as namespace]

# Import from file
from "path/to/file.lambda" import name1, name2, ...
```

### Available Commands

- **Lambda: Reduce Expression** - Reduce current expression to normal form
- **Lambda: Configure LSP Server** - Set or change lambda CLI path
- **Lambda: Toggle Inlay Hints** - Show/hide inline hints

## LSP Server

The extension uses the Lambda LSP server (`lambda lsp`) for:

- **Completion**: Keyword and command suggestions
- **Inlay Hints**: Display reduced forms and test information
- **Hover**: Show documentation for commands
- **Diagnostics**: Real-time error reporting (planned)

## Development & Advanced Usage

For development setup, building from source, and debugging:
→ See the DEVELOPMENT.md file in the repository

## Troubleshooting

### LSP server won't start
1. Check if lambda CLI is installed: `which lambda`
2. Verify lambda works: `lambda --version`
3. Set full path in settings if not in PATH

### No file recognition
- Ensure file has `.lambda` extension
- Check language mode (bottom right of editor)
- Try reloading VSCode

## Features (Roadmap)

- [ ] Real-time reduction preview with side panel
- [ ] Inline test result decorations
- [ ] Jump to definition for imports
- [ ] Code formatting
- [ ] Comprehensive error diagnostics
- [ ] Integration with lambda documentation

## License

MIT
