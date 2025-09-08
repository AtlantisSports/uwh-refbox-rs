#!/usr/bin/env pwsh
# Smart commit script for uwh-refbox-rs
# Usage: .\scripts\commit.ps1 "Your commit message"

param(
    [Parameter(Mandatory=$true, HelpMessage="Commit message")]
    [string]$Message,
    
    [Parameter(HelpMessage="Skip quality checks (not recommended)")]
    [switch]$SkipChecks,
    
    [Parameter(HelpMessage="Skip pushing to remote repository")]
    [switch]$SkipPush
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

if (-not $SkipPush) {
    # Push to remote repository
    Write-Host "🌐 Pushing to remote repository..." -ForegroundColor Cyan
    $currentBranch = git branch --show-current
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Failed to get current branch name." -ForegroundColor Red
        exit 1
    }
    
    git push origin $currentBranch
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Failed to push to remote repository." -ForegroundColor Red
        Write-Host "💡 Commit was successful but push failed. You can manually push later with: git push origin $currentBranch" -ForegroundColor Yellow
        exit 1
    }
    
    Write-Host "🚀 Successfully pushed to remote repository!" -ForegroundColor Green
    Write-Host "🎉 Smart commit complete: committed and pushed '$Message'" -ForegroundColor Green
} else {
    Write-Host "⏭️  Skipping push to remote repository (use -SkipPush to change this behavior)" -ForegroundColor Yellow
}
