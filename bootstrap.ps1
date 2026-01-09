#!/usr/bin/env pwsh
# bootstrap.ps1 - Build/test/bench script for alembic-rs
#
# Commands:
#   test      - Run all tests
#   build     - Build release
#   check     - cargo check + clippy
#   bench     - Benchmark file reading
#   clean     - Clean build artifacts
#   help      - Show this help

param(
    [Parameter(Position=0)]
    [ValidateSet("test", "build", "check", "bench", "clean", "help", "")]
    [string]$Mode = "",
    
    [Alias("v")]
    [switch]$ShowDetails,
    
    [Alias("h", "?")]
    [switch]$Help
)

$ErrorActionPreference = "Continue"
$script:RootDir = $PSScriptRoot

# Test files
$TestFiles = @{
    chess3 = "data/chess3.abc"
    chess4 = "data/chess4.abc"
    bmw    = "data/bmw.abc"
}

# ============================================================
# UTILITY FUNCTIONS
# ============================================================

function Format-Time {
    param([double]$ms)
    if ($ms -lt 1000) { return "{0:N0}ms" -f $ms }
    elseif ($ms -lt 60000) { return "{0:N1}s" -f ($ms / 1000) }
    else {
        $min = [math]::Floor($ms / 60000)
        $sec = ($ms % 60000) / 1000
        return "{0}m{1:N0}s" -f $min, $sec
    }
}

function Write-Header {
    param([string]$Text)
    $line = "=" * 60
    Write-Host ""
    Write-Host $line -ForegroundColor Cyan
    Write-Host $Text -ForegroundColor Cyan
    Write-Host $line -ForegroundColor Cyan
}

function Write-SubHeader {
    param([string]$Text)
    Write-Host ""
    Write-Host "[$Text]" -ForegroundColor Yellow
}

# ============================================================
# HELP
# ============================================================

function Show-Help {
    Write-Host @"

 ALEMBIC-RS BUILD SCRIPT

 COMMANDS
   test      Run all tests (unit + integration)
   build     Build release binary
   check     Run cargo check + clippy
   bench     Benchmark file reading performance
   clean     Clean build artifacts

 OPTIONS
   -v        Show detailed output

 EXAMPLES
   .\bootstrap.ps1 test           # Run all tests
   .\bootstrap.ps1 build          # Build release
   .\bootstrap.ps1 bench          # Benchmark reading
   .\bootstrap.ps1 check          # Check + clippy

"@ -ForegroundColor White
}

# ============================================================
# TEST MODE
# ============================================================

function Invoke-TestMode {
    Write-Header "ALEMBIC-RS TESTS"
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    
    Write-SubHeader "Unit Tests"
    if ($ShowDetails) {
        cargo test --lib -- --nocapture
    } else {
        cargo test --lib
    }
    $unitResult = $LASTEXITCODE
    
    Write-SubHeader "Integration Tests"
    if ($ShowDetails) {
        cargo test --test read_files -- --nocapture
    } else {
        cargo test --test read_files
    }
    $intResult = $LASTEXITCODE
    
    $sw.Stop()
    
    Write-Header "RESULTS"
    Write-Host ""
    
    if ($unitResult -eq 0 -and $intResult -eq 0) {
        Write-Host "  All tests passed!" -ForegroundColor Green
    } else {
        Write-Host "  Some tests failed" -ForegroundColor Red
    }
    Write-Host "  Time: $(Format-Time $sw.ElapsedMilliseconds)" -ForegroundColor Cyan
    Write-Host ""
    
    exit $(if ($unitResult -eq 0 -and $intResult -eq 0) { 0 } else { 1 })
}

# ============================================================
# BUILD MODE
# ============================================================

function Invoke-BuildMode {
    Write-Header "BUILD RELEASE"
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    cargo build --release
    $result = $LASTEXITCODE
    $sw.Stop()
    
    Write-Host ""
    if ($result -eq 0) {
        Write-Host "  Build successful!" -ForegroundColor Green
        Write-Host "  Time: $(Format-Time $sw.ElapsedMilliseconds)" -ForegroundColor Cyan
    } else {
        Write-Host "  Build failed" -ForegroundColor Red
    }
    Write-Host ""
    
    exit $result
}

# ============================================================
# CHECK MODE
# ============================================================

function Invoke-CheckMode {
    Write-Header "CHECK + CLIPPY"
    
    Write-SubHeader "cargo check"
    cargo check
    $checkResult = $LASTEXITCODE
    
    Write-SubHeader "cargo clippy"
    cargo clippy -- -D warnings
    $clippyResult = $LASTEXITCODE
    
    Write-Host ""
    if ($checkResult -eq 0 -and $clippyResult -eq 0) {
        Write-Host "  All checks passed!" -ForegroundColor Green
    } else {
        Write-Host "  Checks failed" -ForegroundColor Red
    }
    Write-Host ""
    
    exit $(if ($checkResult -eq 0 -and $clippyResult -eq 0) { 0 } else { 1 })
}

# ============================================================
# BENCH MODE
# ============================================================

function Invoke-BenchMode {
    Write-Header "BENCHMARK FILE READING"
    
    # Build release first
    Write-SubHeader "Building release"
    cargo build --release --quiet
    
    Write-SubHeader "Reading test files"
    
    foreach ($name in $TestFiles.Keys) {
        $path = Join-Path $script:RootDir $TestFiles[$name]
        if (-not (Test-Path $path)) {
            Write-Host "  $name... " -NoNewline
            Write-Host "SKIP (file not found)" -ForegroundColor Yellow
            continue
        }
        
        $size = (Get-Item $path).Length / 1MB
        Write-Host "  $name ($("{0:N1}" -f $size) MB)... " -NoNewline
        
        # Run benchmark test
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $result = cargo test "test_open_$name" --release --quiet 2>&1
        $sw.Stop()
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "$(Format-Time $sw.ElapsedMilliseconds)" -ForegroundColor Green
        } else {
            Write-Host "FAIL" -ForegroundColor Red
        }
    }
    
    Write-SubHeader "Full geometry scan (BMW)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    cargo test test_bmw_geometry --release -- --nocapture 2>&1 | Select-String "Total|vertices|faces"
    $sw.Stop()
    Write-Host "  Scan time: $(Format-Time $sw.ElapsedMilliseconds)" -ForegroundColor Cyan
    
    Write-Host ""
}

# ============================================================
# CLEAN MODE
# ============================================================

function Invoke-CleanMode {
    Write-Header "CLEAN"
    
    cargo clean
    
    # Also clean test output
    $testOut = Join-Path $script:RootDir "test/out"
    if (Test-Path $testOut) {
        Remove-Item -Recurse -Force $testOut
        Write-Host "  Removed test/out" -ForegroundColor Yellow
    }
    
    Write-Host "  Done!" -ForegroundColor Green
    Write-Host ""
}

# ============================================================
# MAIN
# ============================================================

if ($Help -or $Mode -eq "" -or $Mode -eq "help") {
    Show-Help
    exit 0
}

Push-Location $script:RootDir
try {
    switch ($Mode) {
        "test"  { Invoke-TestMode }
        "build" { Invoke-BuildMode }
        "check" { Invoke-CheckMode }
        "bench" { Invoke-BenchMode }
        "clean" { Invoke-CleanMode }
    }
} finally {
    Pop-Location
}
