#!/usr/bin/env python3
"""
bootstrap.py - Build/test/bench script for alembic-rs.

Cross-platform Python version of bootstrap.ps1.
Single-file solution with no external dependencies (stdlib only).

Commands:
    test      - Run all tests
    build     - Build release
    check     - cargo check + clippy
    bench     - Benchmark file reading
    clean     - Clean build artifacts

Usage:
    python bootstrap.py test
    python bootstrap.py build
    python bootstrap.py bench -v
"""

from __future__ import annotations

import argparse
import os
import platform
import shutil
import subprocess
import sys
import time
from pathlib import Path

# ============================================================
# CONSTANTS
# ============================================================

ROOT_DIR = Path(__file__).parent.resolve()

TEST_FILES = {
    "chess3": "data/chess3.abc",
    "chess4": "data/chess4.abc",
    "bmw": "data/bmw.abc",
}


# ============================================================
# COLORS
# ============================================================

class Colors:
    """ANSI color codes."""
    
    RESET = "\033[0m"
    RED = "\033[91m"
    GREEN = "\033[92m"
    YELLOW = "\033[93m"
    CYAN = "\033[96m"
    WHITE = "\033[97m"
    
    @classmethod
    def init(cls) -> None:
        """Enable ANSI on Windows."""
        if platform.system() == "Windows":
            os.system("")


# ============================================================
# UTILITY FUNCTIONS
# ============================================================

def fmt_time(ms: float) -> str:
    """Format milliseconds nicely."""
    if ms < 1000:
        return f"{ms:.0f}ms"
    elif ms < 60000:
        return f"{ms/1000:.1f}s"
    else:
        mins = int(ms // 60000)
        secs = (ms % 60000) / 1000
        return f"{mins}m{secs:.0f}s"


def print_header(text: str) -> None:
    """Print section header."""
    line = "=" * 60
    print()
    print(f"{Colors.CYAN}{line}")
    print(text)
    print(f"{line}{Colors.RESET}")


def print_subheader(text: str) -> None:
    """Print sub-header."""
    print()
    print(f"{Colors.YELLOW}[{text}]{Colors.RESET}")


def run_cmd(args: list[str], quiet: bool = False, capture: bool = False) -> tuple[int, str]:
    """Run command and return (exit_code, output)."""
    try:
        if capture:
            result = subprocess.run(args, cwd=ROOT_DIR, capture_output=True, text=True)
            return result.returncode, result.stdout + result.stderr
        else:
            result = subprocess.run(args, cwd=ROOT_DIR)
            return result.returncode, ""
    except Exception as e:
        return 1, str(e)


# ============================================================
# COMMANDS
# ============================================================

def cmd_test(verbose: bool = False) -> int:
    """Run all tests."""
    print_header("ALEMBIC-RS TESTS")
    
    start = time.time()
    
    print_subheader("Unit Tests")
    if verbose:
        code1, _ = run_cmd(["cargo", "test", "--lib", "--", "--nocapture"])
    else:
        code1, _ = run_cmd(["cargo", "test", "--lib"])
    
    print_subheader("Integration Tests")
    if verbose:
        code2, _ = run_cmd(["cargo", "test", "--test", "read_files", "--", "--nocapture"])
    else:
        code2, _ = run_cmd(["cargo", "test", "--test", "read_files"])
    
    elapsed = (time.time() - start) * 1000
    
    print_header("RESULTS")
    print()
    if code1 == 0 and code2 == 0:
        print(f"  {Colors.GREEN}All tests passed!{Colors.RESET}")
    else:
        print(f"  {Colors.RED}Some tests failed{Colors.RESET}")
    print(f"  {Colors.CYAN}Time: {fmt_time(elapsed)}{Colors.RESET}")
    print()
    
    return 0 if code1 == 0 and code2 == 0 else 1


def cmd_build() -> int:
    """Build release."""
    print_header("BUILD RELEASE")
    
    start = time.time()
    code, _ = run_cmd(["cargo", "build", "--release"])
    elapsed = (time.time() - start) * 1000
    
    print()
    if code == 0:
        print(f"  {Colors.GREEN}Build successful!{Colors.RESET}")
        print(f"  {Colors.CYAN}Time: {fmt_time(elapsed)}{Colors.RESET}")
    else:
        print(f"  {Colors.RED}Build failed{Colors.RESET}")
    print()
    
    return code


def cmd_check() -> int:
    """Run cargo check + clippy."""
    print_header("CHECK + CLIPPY")
    
    print_subheader("cargo check")
    code1, _ = run_cmd(["cargo", "check"])
    
    print_subheader("cargo clippy")
    code2, _ = run_cmd(["cargo", "clippy", "--", "-D", "warnings"])
    
    print()
    if code1 == 0 and code2 == 0:
        print(f"  {Colors.GREEN}All checks passed!{Colors.RESET}")
    else:
        print(f"  {Colors.RED}Checks failed{Colors.RESET}")
    print()
    
    return 0 if code1 == 0 and code2 == 0 else 1


def cmd_bench() -> int:
    """Benchmark file reading."""
    print_header("BENCHMARK FILE READING")
    
    print_subheader("Building release")
    run_cmd(["cargo", "build", "--release", "--quiet"])
    
    print_subheader("Reading test files")
    
    for name, rel_path in TEST_FILES.items():
        path = ROOT_DIR / rel_path
        if not path.exists():
            print(f"  {name}... {Colors.YELLOW}SKIP (file not found){Colors.RESET}")
            continue
        
        size_mb = path.stat().st_size / (1024 * 1024)
        print(f"  {name} ({size_mb:.1f} MB)... ", end="", flush=True)
        
        start = time.time()
        code, _ = run_cmd(["cargo", "test", f"test_open_{name}", "--release", "--quiet"], capture=True)
        elapsed = (time.time() - start) * 1000
        
        if code == 0:
            print(f"{Colors.GREEN}{fmt_time(elapsed)}{Colors.RESET}")
        else:
            print(f"{Colors.RED}FAIL{Colors.RESET}")
    
    print_subheader("Full geometry scan (BMW)")
    start = time.time()
    code, output = run_cmd(["cargo", "test", "test_bmw_geometry", "--release", "--", "--nocapture"], capture=True)
    elapsed = (time.time() - start) * 1000
    
    # Print summary lines
    for line in output.split("\n"):
        if "Total" in line or "vertices" in line or "faces" in line:
            print(f"  {line.strip()}")
    
    print(f"  {Colors.CYAN}Scan time: {fmt_time(elapsed)}{Colors.RESET}")
    print()
    
    return 0


def cmd_clean() -> int:
    """Clean build artifacts."""
    print_header("CLEAN")
    
    run_cmd(["cargo", "clean"])
    
    test_out = ROOT_DIR / "test" / "out"
    if test_out.exists():
        shutil.rmtree(test_out)
        print(f"  {Colors.YELLOW}Removed test/out{Colors.RESET}")
    
    print(f"  {Colors.GREEN}Done!{Colors.RESET}")
    print()
    
    return 0


def show_help() -> None:
    """Show help message."""
    print("""
 ALEMBIC-RS BUILD SCRIPT

 COMMANDS
   test      Run all tests (unit + integration)
   build     Build release binary
   check     Run cargo check + clippy
   bench     Benchmark file reading performance
   clean     Clean build artifacts

 OPTIONS
   -v, --verbose  Show detailed output

 EXAMPLES
   python bootstrap.py test           # Run all tests
   python bootstrap.py build          # Build release
   python bootstrap.py bench          # Benchmark reading
   python bootstrap.py check          # Check + clippy
""")


# ============================================================
# MAIN
# ============================================================

def main() -> int:
    Colors.init()
    
    parser = argparse.ArgumentParser(
        description="Alembic-rs build script",
        add_help=False,
    )
    parser.add_argument("command", nargs="?", default="help",
                        choices=["test", "build", "check", "bench", "clean", "help"])
    parser.add_argument("-v", "--verbose", action="store_true")
    parser.add_argument("-h", "--help", action="store_true")
    
    args = parser.parse_args()
    
    if args.help or args.command == "help":
        show_help()
        return 0
    
    os.chdir(ROOT_DIR)
    
    if args.command == "test":
        return cmd_test(args.verbose)
    elif args.command == "build":
        return cmd_build()
    elif args.command == "check":
        return cmd_check()
    elif args.command == "bench":
        return cmd_bench()
    elif args.command == "clean":
        return cmd_clean()
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
