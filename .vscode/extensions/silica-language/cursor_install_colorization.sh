#!/bin/bash

# Installation script for Silica syntax highlighting extension in Cursor
# Run this script from the extension directory to install it in Cursor

# Get the directory where this script is located (the extension directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine Cursor extensions directory based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    CURSOR_EXT_DIR="$HOME/.cursor/extensions"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    CURSOR_EXT_DIR="$HOME/.cursor/extensions"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    # Windows (Git Bash or Cygwin)
    CURSOR_EXT_DIR="$USERPROFILE/.cursor/extensions"
else
    echo "❌ Unsupported OS: $OSTYPE"
    echo "Please manually copy this extension to your Cursor extensions folder"
    exit 1
fi

# Create Cursor extensions directory if it doesn't exist
mkdir -p "$CURSOR_EXT_DIR"

# Copy extension
echo "📦 Installing Silica extension..."
echo "   From: $SCRIPT_DIR"
echo "   To:   $CURSOR_EXT_DIR/silica-language"
echo ""

cp -r "$SCRIPT_DIR" "$CURSOR_EXT_DIR/silica-language"

if [ $? -eq 0 ]; then
    echo "✅ Extension installed successfully!"
    echo ""
    echo "📝 Next steps:"
    echo "   1. Reload Cursor (Cmd+Shift+P → 'Developer: Reload Window')"
    echo "   2. Open a .silica file to see syntax highlighting"
    echo ""
    echo "💡 Tip: The extension is now at: $CURSOR_EXT_DIR/silica-language"
else
    echo "❌ Installation failed"
    echo "   Please check permissions and try again"
    exit 1
fi
