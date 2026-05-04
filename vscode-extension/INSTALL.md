# Lambda VSCode Extension - Installation & Quick Start

## 📦 Installation

### File Location
```
/home/syoch/work/lambda/vscode-extension/lambda-lang-support.vsix
```

### Step 1: Install the Extension
1. Open Visual Studio Code
2. Open Extensions view: `Ctrl+Shift+X`
3. Click the `...` menu → "Install from VSIX"
4. Select: `/home/syoch/work/lambda/vscode-extension/lambda-lang-support.vsix`
5. Click "Install"

### Step 2: Reload VSCode
- Press `Ctrl+Shift+P` → Type "Reload Window" → Press Enter
- Or close and reopen VSCode

### Step 3: Configure Lambda CLI Path

**Option A: Via Settings UI**
1. Open Settings: `Ctrl+,`
2. Search: "lambda.lsp.serverPath"
3. Enter the path to your lambda CLI
   - If lambda is in PATH: `lambda`
   - Or use full path: `/home/syoch/work/lambda/target/release/lambda`

**Option B: Via Command Palette**
1. Press `Ctrl+Shift+P`
2. Type: "Lambda: Configure LSP Server"
3. Enter the path when prompted

## ✅ Verification

### Create a Test File
1. Create a new file: `test.lambda`
2. Add this content:
   ```lambda
   from "basics.lambda" import I, K, S

   # Test reduction
   reduce I
   reduce K I
   ```

### Expected Behavior
- File should have syntax highlighting (colors for keywords)
- `.lambda` file extension should be recognized
- LSP server should start in the background
- Output panel should show "Lambda Language Server" messages

## 📝 Usage Examples

### Reduce Command
```lambda
# Reduce an expression to normal form
reduce I
reduce K I
reduce (S K K)
```

### Testing Assertions
```lambda
assert: I x == (\y. y) x
assert: K x y == x
```

### Reduction Steps
```lambda
# Show all reduction steps (first 10 steps)
reduce_steps(10) (S K K) I
```

## ⚙️ Advanced Configuration

### Editor Settings (settings.json)
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

### Custom Keybindings
Add to `keybindings.json`:
```json
{
  "key": "ctrl+shift+r",
  "command": "lambda.reduce",
  "when": "editorLangId == lambda"
},
{
  "key": "ctrl+shift+l",
  "command": "lambda.configureLsp",
  "when": "editorLangId == lambda"
}
```

## 🔧 Troubleshooting

### "Lambda LSP server failed to start"
1. Check if lambda CLI is installed:
   ```bash
   which lambda
   lambda --version
   ```
2. If not in PATH, set full path in settings
3. Check Output panel for error details:
   - Press `Ctrl+Shift+U`
   - Select "Lambda Language Server" channel

### "Cannot find .lambda files"
- Ensure file extension is `.lambda`
- Check language mode (bottom right): should show "Lambda"
- Restart VSCode if needed

### "Syntax highlighting not working"
- Verify extension is activated (check Extensions panel)
- Try reloading the window: `Ctrl+Shift+P` → "Reload Window"

## 📚 Features

- **Syntax Highlighting**: Keywords, comments, strings highlighted
- **Language Recognition**: `.lambda` files automatically recognized
- **LSP Integration**: Real-time language features via lambda CLI
- **Command Palette**: Quick access to extension commands
- **Configurable**: All features can be toggled in settings

## 🚀 Build from Source

### Using npm
```bash
cd /home/syoch/work/lambda/vscode-extension
npm install
npm run compile
npm run package
```

### Using Nix
```bash
cd /home/syoch/work/lambda/vscode-extension
nix develop
npm run package
```

Or directly:
```bash
nix run .#build-vsix
```

### Output
- VSIX file: `lambda-lang-support.vsix`

## 📞 Support

For issues:
1. Check Output panel (`Ctrl+Shift+U`)
2. Review extension logs in "Lambda Language Server" channel
3. Check lambda CLI is working: `lambda --help`
