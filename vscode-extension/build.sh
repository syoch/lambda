#!/bin/bash
# Build script for Lambda VSCode Extension

set -e

echo "Building Lambda VSCode Extension..."

# Check for required tools
if ! command -v npm &> /dev/null; then
    echo "Error: npm is required but not installed"
    exit 1
fi

if ! command -v vsce &> /dev/null; then
    echo "Installing vsce..."
    npm install -g @vscode/vsce
fi

# Install dependencies
echo "Installing dependencies..."
npm install

# Compile TypeScript
echo "Compiling TypeScript..."
npm run compile

# Package as VSIX
echo "Packaging as VSIX..."
npx vsce package --out lambda-lang-support.vsix

echo ""
echo "✓ Build complete!"
echo "VSIX file created: ./lambda-lang-support.vsix"
echo ""
echo "To install in VSCode:"
echo "  1. Open VSCode"
echo "  2. Extensions > ... > Install from VSIX"
echo "  3. Select: ./lambda-lang-support.vsix"
