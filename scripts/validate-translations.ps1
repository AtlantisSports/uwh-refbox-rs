#!/usr/bin/env pwsh
# Translation key validation script
# Checks that all fl!() macro calls have corresponding translations

Write-Host "🔍 Validating translation keys..." -ForegroundColor Cyan

# Find all fl!() calls in Rust files
$flCalls = Select-String -Path "refbox/src/**/*.rs" -Pattern 'fl!\("([^"]+)"' -AllMatches | ForEach-Object {
    $_.Matches | ForEach-Object {
        $_.Groups[1].Value
    }
} | Sort-Object -Unique

# Check English translation file (fallback language)
$enTranslations = Get-Content "refbox/translations/en-US/refbox.ftl" | Where-Object { $_ -match '^([a-zA-Z0-9-_]+)\s*=' } | ForEach-Object {
    ($_ -split '\s*=')[0].Trim()
} | Sort-Object -Unique

Write-Host "Found $($flCalls.Count) unique fl!() calls and $($enTranslations.Count) English translations" -ForegroundColor Yellow

# Find missing keys
$missingKeys = @()
foreach ($key in $flCalls) {
    if ($key -notin $enTranslations) {
        $missingKeys += $key
        Write-Host "❌ Missing key: $key" -ForegroundColor Red
    }
}

if ($missingKeys.Count -eq 0) {
    Write-Host "✅ All translation keys found!" -ForegroundColor Green
} else {
    Write-Host "❌ Found $($missingKeys.Count) missing translation keys" -ForegroundColor Red
    Write-Host "Missing keys:" -ForegroundColor Yellow
    $missingKeys | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
}

# Also check for unused translations
$unusedKeys = @()
foreach ($key in $enTranslations) {
    if ($key -notin $flCalls) {
        $unusedKeys += $key
    }
}

if ($unusedKeys.Count -gt 0) {
    Write-Host "ℹ️  Found $($unusedKeys.Count) potentially unused translations" -ForegroundColor Blue
}
