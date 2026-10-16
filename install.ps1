# Terminal Browser Installer for Windows
param (
    [switch]$SkipBuild = $false
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "      TERMINAL BROWSER (tb) INSTALLER AND AGENT PROVISIONER           " -ForegroundColor Cyan
Write-Host "======================================================================" -ForegroundColor Cyan

# 1. Locate Cargo
$CargoBin = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $CargoBin)) {
    $CargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
    if ($CargoCmd) {
        $CargoBin = $CargoCmd.Source
    } else {
        Write-Error "Cargo toolchain not found. Please install Rust via https://rustup.rs"
        exit 1
    }
}

# 2. Build Release Binary
$ReleaseExe = Join-Path $ScriptDir "target\release\terminal-browser.exe"
if (-not $SkipBuild -or -not (Test-Path $ReleaseExe)) {
    Write-Host ""
    Write-Host "[1/4] Compiling optimized release binary (Fat LTO, stripped)..." -ForegroundColor Yellow
    $env:RUSTC_WRAPPER = ""
    & $CargoBin build --release --manifest-path (Join-Path $ScriptDir "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed"
        exit 1
    }
} else {
    Write-Host ""
    Write-Host "[1/4] Using existing binary: $ReleaseExe" -ForegroundColor DarkGray
}

if (-not (Test-Path $ReleaseExe)) {
    Write-Error "Release binary not found at $ReleaseExe"
    exit 1
}

# 3. Install Executables into PATH
Write-Host ""
Write-Host "[2/4] Installing executables into System PATH..." -ForegroundColor Yellow

$InstallTargets = @()
$CargoDir = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $CargoDir) {
    $InstallTargets += $CargoDir
}

$ScoopShims = "$env:USERPROFILE\scoop\shims"
if (Test-Path $ScoopShims) {
    $InstallTargets += $ScoopShims
}

$Aliases = @("terminal-browser.exe", "term-browser.exe", "tb.exe")

foreach ($targetDir in $InstallTargets) {
    foreach ($alias in $Aliases) {
        $dest = Join-Path $targetDir $alias
        Copy-Item -Path $ReleaseExe -Destination $dest -Force
        Write-Host "  Installed: $dest" -ForegroundColor Green
    }
}

# 4. Provision Antigravity Skills and Rules
Write-Host ""
Write-Host "[3/4] Provisioning Antigravity System Skills and Rules..." -ForegroundColor Yellow

$SkillSource = Join-Path $ScriptDir ".agents\skills\terminal-browser\SKILL.md"
$SkillDestDir = "$env:USERPROFILE\.gemini\config\skills\terminal-browser"
if (Test-Path $SkillSource) {
    if (-not (Test-Path $SkillDestDir)) {
        New-Item -ItemType Directory -Path $SkillDestDir -Force | Out-Null
    }
    Copy-Item -Path $SkillSource -Destination (Join-Path $SkillDestDir "SKILL.md") -Force
    Write-Host "  Provisioned Skill: $SkillDestDir\SKILL.md" -ForegroundColor Green
}

$RuleSource = Join-Path $ScriptDir ".agents\rules\terminal_browser_standard.md"
$RuleDestDir = "$env:USERPROFILE\.gemini\config\rules"
if (Test-Path $RuleSource) {
    if (-not (Test-Path $RuleDestDir)) {
        New-Item -ItemType Directory -Path $RuleDestDir -Force | Out-Null
    }
    Copy-Item -Path $RuleSource -Destination (Join-Path $RuleDestDir "terminal_browser_standard.md") -Force
    Write-Host "  Provisioned Rule:  $RuleDestDir\terminal_browser_standard.md" -ForegroundColor Green
}

# 5. Verify Installation
Write-Host ""
Write-Host "[4/4] Verifying CLI functionality..." -ForegroundColor Yellow
$TbCmd = Join-Path $CargoDir "tb.exe"
if (-not (Test-Path $TbCmd) -and (Test-Path (Join-Path $ScoopShims "tb.exe"))) {
    $TbCmd = Join-Path $ScoopShims "tb.exe"
}

$ver = & $TbCmd --version
Write-Host "  CLI Verified: $ver" -ForegroundColor Green

Write-Host ""
Write-Host "======================================================================" -ForegroundColor Cyan
Write-Host "INSTALLATION COMPLETE! You can now run:" -ForegroundColor Green
Write-Host "  tb fetch https://docs.anthropic.com/llms.txt" -ForegroundColor White
Write-Host "  tb tui https://news.ycombinator.com" -ForegroundColor White
Write-Host "  tb mcp" -ForegroundColor White
Write-Host "======================================================================" -ForegroundColor Cyan
