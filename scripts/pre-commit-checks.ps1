#!/usr/bin/env pwsh
# Pre-commit quality checks script for uwh-refbox-rs
# Run this script before committing to ensure code quality

Write-Host "🚀 Running pre-commit quality checks..." -ForegroundColor Cyan

# 1. Check code formatting
Write-Host "📝 Checking code formatting..." -ForegroundColor Yellow
cargo fmt --check
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Code formatting check failed. Running cargo fmt to fix..." -ForegroundColor Red
    cargo fmt
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Failed to format code. Please fix manually." -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ Code formatted successfully" -ForegroundColor Green
} else {
    Write-Host "✅ Code formatting is correct" -ForegroundColor Green
}

# 2. Run clippy linter
Write-Host "🔍 Running clippy linter..." -ForegroundColor Yellow
cargo clippy
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Clippy checks failed. Please fix the issues above." -ForegroundColor Red
    exit 1
}
Write-Host "✅ Clippy checks passed" -ForegroundColor Green

# 3. Build the project
Write-Host "🔨 Building project..." -ForegroundColor Yellow
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed. Please fix the compilation errors." -ForegroundColor Red
    exit 1
}
Write-Host "✅ Build successful" -ForegroundColor Green

# 4. Run tests
Write-Host "🧪 Running tests..." -ForegroundColor Yellow
cargo test
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Tests failed. Please fix the failing tests." -ForegroundColor Red
    exit 1
}
Write-Host "✅ All tests passed" -ForegroundColor Green

# 5. Validate translations
Write-Host "🌐 Validating translations..." -ForegroundColor Yellow
& "$PSScriptRoot\validate-translations.ps1"
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Translation validation failed." -ForegroundColor Red
    exit 1
}
Write-Host "✅ Translation validation passed" -ForegroundColor Green

Write-Host "🎉 All pre-commit checks passed! Ready to commit." -ForegroundColor Green
