#!/bin/sh
set -e

BINARY="acil"
REPO="a666hn/acil"
GITHUB="https://github.com/${REPO}"

detect_platform() {
  platform="$(uname -s | tr '[:upper:]' '[:lower:]')"
  case "${platform}" in
    msys_nt*|cygwin_nt*|mingw*) platform="pc-windows-msvc" ;;
    linux*) platform="unknown-linux-musl" ;;
    darwin) platform="apple-darwin" ;;
    *)
      err "Unsupported platform: ${platform}"
      ;;
  esac
  printf '%s' "${platform}"
}

detect_arch() {
  arch="$(uname -m | tr '[:upper:]' '[:lower:]')"
  case "${arch}" in
    x86_64|amd64) arch="x86_64" ;;
    arm64|aarch64) arch="aarch64" ;;
    *)
      err "Unsupported architecture: ${arch}"
      ;;
  esac
  printf '%s' "${arch}"
}

has() {
  command -v "$1" >/dev/null 2>&1
}

err() {
  printf "error: %s\n" "$1" >&2
  exit 1
}

need_cmd() {
  if ! has "$1"; then
    err "Required command '$1' not found. Please install it."
  fi
}

download() {
  url="$1"
  output="$2"

  if has curl; then
    curl --fail --silent --location --output "$output" "$url" || err "Download failed: $url"
  elif has wget; then
    wget --quiet --output-document="$output" "$url" || err "Download failed: $url"
  else
    err "Neither 'curl' nor 'wget' found. Please install one."
  fi
}

get_latest_version() {
  version=""
  if has curl; then
    response=$(curl --silent --fail "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null) || true
    if [ -n "$response" ]; then
      version=$(printf '%s' "$response" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/') || true
    fi
  elif has wget; then
    response=$(wget --quiet -O- "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null) || true
    if [ -n "$response" ]; then
      version=$(printf '%s' "$response" | grep '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/') || true
    fi
  fi

  if [ -z "$version" ]; then
    err "Could not determine latest version. Please specify with --version."
  fi

  printf '%s' "$version"
}

main() {
  need_cmd uname

  # Parse arguments
  VERSION=""
  BIN_DIR=""

  while [ $# -gt 0 ]; do
    case "$1" in
      --version)
        VERSION="$2"
        shift 2
        ;;
      --bin-dir)
        BIN_DIR="$2"
        shift 2
        ;;
      *)
        err "Unknown option: $1"
        ;;
    esac
  done

  # Detect platform and arch
  PLATFORM="$(detect_platform)"
  ARCH="$(detect_arch)"
  TARGET="${ARCH}-${PLATFORM}"

  # Get version
  if [ -z "$VERSION" ]; then
    printf "Fetching latest version...\n"
    VERSION="$(get_latest_version)"
  fi

  # Determine file extension and binary name
  if [ "$PLATFORM" = "pc-windows-msvc" ]; then
    EXT="zip"
    BINARY_NAME="${BINARY}.exe"
  else
    EXT="tar.gz"
    BINARY_NAME="${BINARY}"
  fi

  ARCHIVE="${BINARY}-${VERSION}-${TARGET}.${EXT}"
  DOWNLOAD_URL="${GITHUB}/releases/download/${VERSION}/${ARCHIVE}"

  # Create temp directory
  TMPDIR="$(mktemp -d)"
  trap 'rm -rf "$TMPDIR"' EXIT

  # Download
  printf "Downloading %s %s (%s)...\n" "$BINARY" "$VERSION" "$TARGET"
  download "$DOWNLOAD_URL" "$TMPDIR/${ARCHIVE}"

  # Extract
  printf "Extracting...\n"
  cd "$TMPDIR"
  if [ "$EXT" = "tar.gz" ]; then
    tar xzf "$ARCHIVE"
  elif [ "$EXT" = "zip" ]; then
    if has unzip; then
      unzip -q "$ARCHIVE"
    elif has 7z; then
      7z x "$ARCHIVE" >/dev/null
    else
      err "Neither 'unzip' nor '7z' found. Please install one."
    fi
  fi

  # Find the binary in extracted directory
  EXTRACTED_DIR="${BINARY}-${VERSION}-${TARGET}"

  # Determine install directory
  if [ -z "$BIN_DIR" ]; then
    if [ -w "/usr/local/bin" ]; then
      BIN_DIR="/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
      BIN_DIR="$HOME/.local/bin"
    else
      mkdir -p "$HOME/.local/bin"
      BIN_DIR="$HOME/.local/bin"
    fi
  fi

  # Install
  printf "Installing to %s/%s...\n" "$BIN_DIR" "$BINARY"
  if [ -w "$BIN_DIR" ]; then
    cp "${EXTRACTED_DIR}/${BINARY_NAME}" "$BIN_DIR/${BINARY}"
    chmod +x "$BIN_DIR/${BINARY}"
  else
    printf "Need sudo to install to %s\n" "$BIN_DIR"
    sudo cp "${EXTRACTED_DIR}/${BINARY_NAME}" "$BIN_DIR/${BINARY}"
    sudo chmod +x "$BIN_DIR/${BINARY}"
  fi

  # Verify
  printf "Verifying...\n"
  if "$BIN_DIR/${BINARY}" --version >/dev/null 2>&1; then
    printf "%s %s installed successfully!\n" "$BINARY" "$("$BIN_DIR/${BINARY}" --version 2>/dev/null | head -1)"
  else
    printf "Installed to %s/%s\n" "$BIN_DIR" "$BINARY"
    printf "Note: You may need to add %s to your PATH\n" "$BIN_DIR"
  fi
}

main "$@"
