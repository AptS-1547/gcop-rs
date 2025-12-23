"""
gcop-rs: AI-powered Git commit message generator and code reviewer.

This is a Python wrapper that downloads and runs the pre-compiled Rust binary.
"""

import os
import platform
import stat
import subprocess
import sys
import urllib.request
from pathlib import Path

__version__ = "0.4.2"

GITHUB_RELEASE_URL = "https://github.com/AptS-1547/gcop-rs/releases/download"


def get_binary_name() -> tuple[str, str]:
    """Get the binary name for the current platform."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    # Normalize machine architecture
    if machine in ("x86_64", "amd64"):
        arch = "amd64"
    elif machine in ("arm64", "aarch64"):
        arch = "arm64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")

    # Determine platform string
    if system == "darwin":
        platform_str = f"macos-{arch}"
        binary_name = "gcop-rs"
    elif system == "linux":
        platform_str = f"linux-{arch}"
        binary_name = "gcop-rs"
    elif system == "windows":
        if arch == "arm64":
            platform_str = "windows-aarch64.exe"
        else:
            platform_str = "windows-amd64.exe"
        binary_name = "gcop-rs.exe"
    else:
        raise RuntimeError(f"Unsupported operating system: {system}")

    return platform_str, binary_name


def get_binary_path() -> Path:
    """Get the path where the binary should be stored."""
    # Store in user's cache directory
    if platform.system() == "Windows":
        cache_dir = Path(os.environ.get("LOCALAPPDATA", Path.home() / "AppData" / "Local"))
    elif platform.system() == "Darwin":
        cache_dir = Path.home() / "Library" / "Caches"
    else:
        cache_dir = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))

    return cache_dir / "gcop-rs" / "bin"


def download_binary() -> Path:
    """Download the binary for the current platform."""
    platform_str, binary_name = get_binary_name()
    bin_dir = get_binary_path()
    bin_dir.mkdir(parents=True, exist_ok=True)

    binary_path = bin_dir / binary_name
    version_file = bin_dir / "version"

    # Check if we already have the correct version
    if binary_path.exists() and version_file.exists():
        current_version = version_file.read_text().strip()
        if current_version == __version__:
            return binary_path

    # Download the binary
    url = f"{GITHUB_RELEASE_URL}/v{__version__}/gcop-rs-v{__version__}-{platform_str}"
    print(f"Downloading gcop-rs v{__version__} for {platform_str}...", file=sys.stderr)

    try:
        urllib.request.urlretrieve(url, binary_path)
    except Exception as e:
        raise RuntimeError(f"Failed to download binary from {url}: {e}") from e

    # Make executable on Unix
    if platform.system() != "Windows":
        binary_path.chmod(binary_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    # Save version
    version_file.write_text(__version__)

    print(f"Downloaded to {binary_path}", file=sys.stderr)
    return binary_path


def main() -> int:
    """Main entry point - download binary if needed and run it."""
    try:
        binary_path = download_binary()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1

    # Run the binary with all arguments
    try:
        result = subprocess.run([str(binary_path)] + sys.argv[1:])
        return result.returncode
    except KeyboardInterrupt:
        return 130


if __name__ == "__main__":
    sys.exit(main())
