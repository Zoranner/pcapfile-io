# Git Hooks Install Script

$ErrorActionPreference = "Stop"

Write-Host "Installing Git Hooks..." -ForegroundColor Cyan

# Check if we're in a git repository
if (-not (Test-Path ".git")) {
    Write-Host "Error: Not a git repository" -ForegroundColor Red
    exit 1
}

# Check if we're in the right project directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "Error: Cargo.toml not found" -ForegroundColor Red
    exit 1
}

$hooksDir = ".git\hooks"

# Ensure .git/hooks directory exists
if (-not (Test-Path $hooksDir)) {
    New-Item -ItemType Directory -Path $hooksDir -Force | Out-Null
}

# Install pre-commit hook with embedded logic
$preCommitTarget = Join-Path $hooksDir "pre-commit"

# Create pre-commit hook with embedded logic
$preCommitContent = @"
#!/bin/sh
# Git pre-commit hook for pcapfile-io
# 
# This hook runs the following checks before each commit:
# 1. cargo fmt --all -- --check (code formatting)
# 2. cargo clippy --all-targets --all-features -- -D warnings (code quality)
#
# If any check fails, the commit will be blocked.

set -e

echo "Running pre-commit checks..."

# Change to repository root
cd "`$(dirname "`$0")/../.."

# Check if we're in a Rust project directory
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Cargo.toml not found"
    echo "Please ensure you're running this script in the Rust project root directory."
    exit 1
fi

echo ""
echo "Checking code formatting..."

# Check code formatting
if ! cargo fmt --all -- --check > /dev/null 2>&1; then
    echo ""
    echo "Code formatting check failed"
    echo "Tip: Run 'cargo fmt --all' to fix formatting issues automatically"
    cargo fmt --all -- --check
    exit 1
fi
echo "Code formatting check passed"

echo ""
echo "Running Clippy code quality check..."

# Run Clippy check
if ! cargo clippy --all-targets --all-features -- -D warnings > /dev/null 2>&1; then
    echo ""
    echo "Clippy check failed"
    echo "Tip: Please fix the above Clippy warnings before committing"
    cargo clippy --all-targets --all-features -- -D warnings
    exit 1
fi
echo "Clippy check passed"

echo ""
echo "All pre-commit checks passed! Proceeding with commit..."
echo ""

exit 0
"@

$preCommitContent | Out-File -FilePath $preCommitTarget -Encoding ASCII

Write-Host "Pre-commit hook installed successfully" -ForegroundColor Green

Write-Host ""
Write-Host "Git Hooks installation complete!" -ForegroundColor Green
Write-Host "Pre-commit checks will now run automatically on each commit." -ForegroundColor Cyan
Write-Host ""