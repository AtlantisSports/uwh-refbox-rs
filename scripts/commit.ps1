#!/usr/bin/env pwsh
# Smart commit script for uwh-refbox-rs
# Usage: .\scripts\commit.ps1 "Your commit message"

param(
    [Parameter(Mandatory=$true, HelpMessage="Commit message")]
    [string]$Message,
    
    [Parameter(HelpMessage="Skip quality checks (not recommended)")]
    [switch]$SkipChecks
)

if (-not $SkipChecks) {
    Write-Host "🚀 Running pre-commit quality checks..." -ForegroundColor Cyan
    
    # Run the pre-commit checks
    & "$PSScriptRoot\pre-commit-checks.ps1"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Pre-commit checks failed. Commit aborted." -ForegroundColor Red
        exit 1
    }
}

# Stage all changes
Write-Host "📋 Staging all changes..." -ForegroundColor Yellow
git add .
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to stage changes." -ForegroundColor Red
    exit 1
}

# Commit with the provided message
Write-Host "💾 Committing changes..." -ForegroundColor Yellow
git commit -m $Message
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Commit failed." -ForegroundColor Red
    exit 1
}

Write-Host "🎉 Successfully committed with message: '$Message'" -ForegroundColor Green
