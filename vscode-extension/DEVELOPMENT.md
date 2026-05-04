# Lambda VSCode Extension - Development & Installation

## Quick Start

### Option 1: Using Nix (Recommended)

```bash
cd vscode-extension

# Enter the dev shell
nix develop

# Build the VSIX
nix run .#build-vsix
# or
npm run compile && npx vsce package
```

### Option 2: Manual Build

```bash
cd vscode-extension

# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Package as VSIX
npm run package
```

### Option 3: Using Build Script

```bash
cd vscode-extension
bash ./build.sh
```

## Installation

1. **Build the VSIX** (see above)

2. **Install in VSCode**:
   - Open VSCode
   - Press `Ctrl+Shift+X` (Extensions)
   - Click `...` menu → "Install from VSIX"
   - Select `lambda-lang-support.vsix`

3. **Configure Lambda CLI Path**:
   - Open VSCode Settings (`Ctrl+,`)
   - Search for "lambda.lsp.serverPath"
   - Set to your lambda CLI path (default: `lambda`)
   - Or use command: `Lambda: Configure LSP Server`

## Configuration

### Settings (in VSCode Settings)

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

### Via Command Palette

1. Press `Ctrl+Shift+P`
2. Type "Lambda: Configure LSP Server"
3. Enter the path to lambda CLI

## Troubleshooting

### "Lambda LSP server failed to start"

**Check if lambda is in PATH**:
```bash
which lambda
lambda --version
```

**Set full path in settings**:
- Settings → "lambda.lsp.serverPath"
- Enter full path: `/usr/local/bin/lambda` or `C:\path\to\lambda.exe`

### "No language support"

- Ensure `.lambda` file extension is recognized
- Check file language mode (bottom right: "Lambda")
- Restart VSCode if needed

### Enable Debug Logging

- Open Output panel (`Ctrl+Shift+U`)
- Select "Lambda Language Server" channel
- Check for error messages

## Development

### Watch Mode

```bash
cd vscode-extension
npm run watch
```

### Debug Extension

1. Open `vscode-extension` folder in VSCode
2. Press `F5` to start debugging
3. A new VSCode window opens with the extension

### Run Tests

```bash
npm run test
```

## Publishing to Marketplace

### Prerequisites

1. Create [Visual Studio Code marketplace account](https://marketplace.visualstudio.com)
2. Create personal access token
3. Install vsce: `npm install -g @vscode/vsce`

### Publish

```bash
# Login
vsce login <publisher-name>

# Publish
npm run publish
```

## Files Structure

```
vscode-extension/
  ├── src/
  │   └── extension.ts          (Main extension code)
  ├── syntaxes/
  │   └── lambda.tmLanguage.json  (Syntax highlighting)
  ├── package.json              (Manifest)
  ├── tsconfig.json             (TypeScript config)
  ├── language-configuration.json (Language settings)
  ├── flake.nix                 (Nix build config)
  ├── build.sh                  (Build script)
  └── README.md                 (Documentation)
```

## Tips

### Auto-complete for `.lambda` Files

The extension adds `.lambda` file support with:
- Syntax highlighting
- Code completion for keywords
- Hover information
- LSP features (when server is running)

### Custom Keybindings

Add to `keybindings.json`:
```json
{
  "key": "ctrl+shift+r",
  "command": "lambda.reduce",
  "when": "editorLangId == lambda"
}
```

## Support

For issues or questions:
1. Check the Output panel (`Ctrl+Shift+U`)
2. Review [LSP server logs](../src/lsp.rs)
3. Create an issue in the repository
