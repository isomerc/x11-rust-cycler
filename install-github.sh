#!/bin/bash
# Nicotine - One-line installer
# Usage: curl -sSL https://raw.githubusercontent.com/isomerc/nicotine/main/install-github.sh | bash

set -e

REPO="isomerc/nicotine" # Update this with your GitHub username
INSTALL_DIR="$HOME/.local/bin"
BINARY_NAME="nicotine"

echo "=== Nicotine Installer ==="
echo

# Detect architecture
ARCH=$(uname -m)
case $ARCH in
x86_64)
  ARCH="x86_64"
  ;;
aarch64 | arm64)
  ARCH="aarch64"
  ;;
*)
  echo "Unsupported architecture: $ARCH"
  exit 1
  ;;
esac

echo "[1/4] Detecting latest release..."
LATEST_URL=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep "browser_download_url.*nicotine-linux-$ARCH\"" | cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
  echo "Error: Could not find release for linux-$ARCH"
  echo "Looking for: nicotine-linux-$ARCH"
  exit 1
fi

echo "[2/4] Downloading nicotine..."
mkdir -p "$INSTALL_DIR"
curl -sL "$LATEST_URL" -o "/tmp/$BINARY_NAME"
chmod +x "/tmp/$BINARY_NAME"

echo "[3/4] Installing to $INSTALL_DIR..."
mv "/tmp/$BINARY_NAME" "$INSTALL_DIR/"

# Add to PATH if needed
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo "[4/4] Adding $INSTALL_DIR to PATH..."
  SHELL_RC=""
  if [ -n "$BASH_VERSION" ]; then
    SHELL_RC="$HOME/.bashrc"
  elif [ -n "$ZSH_VERSION" ]; then
    SHELL_RC="$HOME/.zshrc"
  fi

  if [ -n "$SHELL_RC" ] && [ -f "$SHELL_RC" ]; then
    if ! grep -q "export PATH.*$INSTALL_DIR" "$SHELL_RC" 2>/dev/null; then
      echo "" >>"$SHELL_RC"
      echo "# Nicotine" >>"$SHELL_RC"
      echo "export PATH=\"\$HOME/.local/bin:\$PATH\"" >>"$SHELL_RC"
      echo "Added to $SHELL_RC"
    fi
  fi
else
  echo "[4/4] PATH already configured"
fi

echo
echo "✓ Installation complete!"
echo
echo "Quick start:"
echo "  nicotine start"
echo
echo "Config will be auto-generated at: ~/.config/nicotine/config.toml"
echo
echo "Note: Restart your terminal first if PATH was just updated"
echo
echo "For autostart on login, create systemd service:"
echo "  mkdir -p ~/.config/systemd/user"
echo "  cat > ~/.config/systemd/user/nicotine.service << 'EOF'"
echo "[Unit]"
echo "Description=Nicotine - EVE Online Multiboxing"
echo "After=graphical-session.target"
echo ""
echo "[Service]"
echo "Type=simple"
echo "ExecStart=%h/.local/bin/nicotine start"
echo "Restart=on-failure"
echo ""
echo "[Install]"
echo "WantedBy=default.target"
echo "EOF"
echo "  systemctl --user enable --now nicotine"
