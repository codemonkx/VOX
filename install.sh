#!/usr/bin/env bash
set -euo pipefail

BIN_DIR="${HOME}/.local/bin"
PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "==> Building vox..."
cd "$PROJECT_DIR"
cargo build --release

echo "==> Installing to ${BIN_DIR}/vox..."
mkdir -p "$BIN_DIR"
ln -sf "${PROJECT_DIR}/target/release/vox" "${BIN_DIR}/vox"

echo "==> Done!"
echo ""
echo "  Type 'vox' to launch the TUI."
echo "  Type 'vox scan ~/Music' to scan your library."
echo ""
echo "  Make sure ${BIN_DIR} is in your PATH."
echo "  If not, add this to your ~/.bashrc or ~/.zshrc:"
echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
