# BMW Roundtrip Test Script
# Reads BMW.abc, writes to new file, compares, shows object list

param(
    [string]$InputFile = "data/bmw.abc",
    [string]$OutputFile = "test_output/bmw_roundtrip.abc"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $ProjectRoot) { $ProjectRoot = "C:\projects\projects.rust\_done\alembic-rs" }

Push-Location $ProjectRoot

try {
    Write-Host "=== BMW Roundtrip Test ===" -ForegroundColor Cyan
    Write-Host ""
    
    # Build release
    Write-Host "[1/5] Building release..." -ForegroundColor Yellow
    cargo build --release 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed"
    }
    Write-Host "  OK" -ForegroundColor Green
    
    # Check input file
    if (-not (Test-Path $InputFile)) {
        throw "Input file not found: $InputFile"
    }
    $inputSize = (Get-Item $InputFile).Length
    Write-Host "[2/5] Input: $InputFile ($([math]::Round($inputSize/1MB, 2)) MB)" -ForegroundColor Yellow
    
    # Create output directory
    $outputDir = Split-Path -Parent $OutputFile
    if ($outputDir -and -not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    
    # Run roundtrip test
    Write-Host "[3/5] Running roundtrip..." -ForegroundColor Yellow
    $result = cargo test test_bmw_roundtrip --release -- --nocapture 2>&1
    $testOutput = $result -join "`n"
    
    if ($testOutput -match "PASSED") {
        Write-Host "  Roundtrip PASSED" -ForegroundColor Green
    } else {
        Write-Host "  Roundtrip FAILED" -ForegroundColor Red
        Write-Host $testOutput
    }
    
    # Extract stats
    if ($testOutput -match "Original BMW: (\d+) xforms, (\d+) meshes, (\d+) vertices") {
        $origXforms = $matches[1]
        $origMeshes = $matches[2]
        $origVerts = $matches[3]
    }
    if ($testOutput -match "Roundtrip: (\d+) xforms, (\d+) meshes, (\d+) vertices") {
        $rtXforms = $matches[1]
        $rtMeshes = $matches[2]
        $rtVerts = $matches[3]
    }
    
    Write-Host ""
    Write-Host "[4/5] Comparison:" -ForegroundColor Yellow
    Write-Host "  +--------------+----------+----------+--------+" -ForegroundColor DarkGray
    Write-Host "  | Metric       | Original | Roundtrip| Match  |" -ForegroundColor DarkGray
    Write-Host "  +--------------+----------+----------+--------+" -ForegroundColor DarkGray
    
    $xformMatch = if ($origXforms -eq $rtXforms) { "YES" } else { "NO" }
    $meshMatch = if ($origMeshes -eq $rtMeshes) { "YES" } else { "NO" }
    $vertMatch = if ($origVerts -eq $rtVerts) { "YES" } else { "NO" }
    
    $xformColor = if ($xformMatch -eq "YES") { "Green" } else { "Red" }
    $meshColor = if ($meshMatch -eq "YES") { "Green" } else { "Red" }
    $vertColor = if ($vertMatch -eq "YES") { "Green" } else { "Red" }
    
    Write-Host ("  | Xforms       | {0,8} | {1,8} | " -f $origXforms, $rtXforms) -NoNewline
    Write-Host ("{0,-6} |" -f $xformMatch) -ForegroundColor $xformColor
    Write-Host ("  | Meshes       | {0,8} | {1,8} | " -f $origMeshes, $rtMeshes) -NoNewline  
    Write-Host ("{0,-6} |" -f $meshMatch) -ForegroundColor $meshColor
    Write-Host ("  | Vertices     | {0,8} | {1,8} | " -f $origVerts, $rtVerts) -NoNewline
    Write-Host ("{0,-6} |" -f $vertMatch) -ForegroundColor $vertColor
    Write-Host "  +--------------+----------+----------+--------+" -ForegroundColor DarkGray
    
    # Show object hierarchy using CLI
    Write-Host ""
    Write-Host "[5/5] Object hierarchy (first 50):" -ForegroundColor Yellow
    
    # Use the alembic CLI to show info
    $cliPath = "target\release\alembic-cli.exe"
    if (Test-Path $cliPath) {
        & $cliPath info $InputFile 2>&1 | Select-Object -First 80
    } else {
        # Fallback: run cargo directly
        cargo run --release -- info $InputFile 2>&1 | Select-Object -First 80
    }
    
    Write-Host ""
    Write-Host "=== Done ===" -ForegroundColor Cyan
    
} catch {
    Write-Host "ERROR: $_" -ForegroundColor Red
    exit 1
} finally {
    Pop-Location
}
