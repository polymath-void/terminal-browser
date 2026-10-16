#!/usr/bin/env bash
# ==============================================================================
# Terminal Browser (tb) Installer for Linux / macOS / Android Termux
# ==============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "======================================================================"
echo "      TERMINAL BROWSER (tb) INSTALLER & AGENT PROVISIONER             "
echo "======================================================================"

# 1. Check for Cargo
if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "Error: Cargo not found. Please install Rust via https://rustup.rs"
        exit 1
    fi
fi

# 2. Build Release Binary
echo ""
echo "[1/4] Compiling optimized release binary..."
RUSTC_WRAPPER="" cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

RELEASE_BIN="$SCRIPT_DIR/target/release/tb"
if [ ! -f "$RELEASE_BIN" ]; then
    RELEASE_BIN="$SCRIPT_DIR/target/release/terminal-browser"
fi
if [ ! -f "$RELEASE_BIN" ]; then
    echo "Error: Release binary not found at $RELEASE_BIN"
    exit 1
fi

# 3. Install Executables
echo ""
echo "[2/4] Installing executables into PATH..."
INSTALL_DIR="$HOME/.cargo/bin"
if [ ! -d "$INSTALL_DIR" ]; then
    INSTALL_DIR="/usr/local/bin"
fi

cp -f "$RELEASE_BIN" "$INSTALL_DIR/terminal-browser"
chmod +x "$INSTALL_DIR/terminal-browser"
ln -sf "$INSTALL_DIR/terminal-browser" "$INSTALL_DIR/term-browser"
ln -sf "$INSTALL_DIR/terminal-browser" "$INSTALL_DIR/tb"
echo "  ✓ Installed: $INSTALL_DIR/terminal-browser"
echo "  ✓ Linked:    $INSTALL_DIR/tb"

# 4. Provision Antigravity Skills & Rules
echo ""
echo "[3/4] Provisioning Antigravity System Skills & Rules..."
SKILL_SRC="$SCRIPT_DIR/.agents/skills/terminal-browser/SKILL.md"
SKILL_DEST="$HOME/.gemini/config/skills/terminal-browser"
if [ -f "$SKILL_SRC" ]; then
    mkdir -p "$SKILL_DEST"
    cp -f "$SKILL_SRC" "$SKILL_DEST/SKILL.md"
    echo "  ✓ Provisioned Skill: $SKILL_DEST/SKILL.md"
fi

RULE_SRC="$SCRIPT_DIR/.agents/rules/terminal_browser_standard.md"
RULE_DEST="$HOME/.gemini/config/rules"
if [ -f "$RULE_SRC" ]; then
    mkdir -p "$RULE_DEST"
    cp -f "$RULE_SRC" "$RULE_DEST/terminal_browser_standard.md"
    echo "  ✓ Provisioned Rule:  $RULE_DEST/terminal_browser_standard.md"
fi

# 5. Verification
echo ""
echo "[4/4] Verifying CLI functionality..."
"$INSTALL_DIR/tb" --version
echo "======================================================================"
echo "INSTALLATION COMPLETE! You can now run:"
echo "  tb fetch https://docs.anthropic.com/llms.txt"
echo "  tb tui https://news.ycombinator.com"
echo "======================================================================"
