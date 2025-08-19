# Font Sizing Test Runner Script
# This script makes it easy to run the CSV-based font sizing tests

Write-Host "🎯 UWH RefBox Font Sizing Test Runner" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

# Check if CSV file exists
$csvFile = "refbox/test_data/font_sizing_test_data.csv"
if (Test-Path $csvFile) {
    Write-Host "✅ Found CSV test data file: $csvFile" -ForegroundColor Green
    
    # Show first few lines of CSV for reference
    Write-Host ""
    Write-Host "📋 Current test data preview:" -ForegroundColor Yellow
    Get-Content $csvFile | Select-Object -First 10 | ForEach-Object {
        if ($_ -match "^#") {
            Write-Host "  $_" -ForegroundColor Gray
        } else {
            Write-Host "  $_" -ForegroundColor White
        }
    }
    Write-Host "  ... (see full file for all test cases)" -ForegroundColor Gray
} else {
    Write-Host "❌ CSV test data file not found: $csvFile" -ForegroundColor Red
    Write-Host "Please ensure you're running this script from the repository root." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "🧪 Available Test Commands:" -ForegroundColor Yellow
Write-Host "1. Individual CSV test cases (shows detailed results for each test)"
Write-Host "2. Group font sizing test (shows how multiple referee names interact)"
Write-Host "3. Manual testing instructions"
Write-Host "4. All font sizing tests"
Write-Host ""

$choice = Read-Host "Enter your choice (1-4) or press Enter for option 1"

switch ($choice) {
    "1" {
        Write-Host "🔍 Running individual CSV test cases..." -ForegroundColor Green
        cargo test test_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
    }
    "2" {
        Write-Host "👥 Running group font sizing test..." -ForegroundColor Green
        cargo test test_group_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
    }
    "3" {
        Write-Host "📖 Showing manual testing instructions..." -ForegroundColor Green
        cargo test test_manual_csv_update_instructions --package refbox --bin refbox -- --nocapture --exact
    }
    "4" {
        Write-Host "🚀 Running all font sizing tests..." -ForegroundColor Green
        Write-Host "Running CSV data test..." -ForegroundColor Yellow
        cargo test test_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
        Write-Host "Running group sizing test..." -ForegroundColor Yellow
        cargo test test_group_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
        Write-Host "Running demo test..." -ForegroundColor Yellow
        cargo test test_csv_based_font_sizing_demo --package refbox --bin refbox -- --nocapture --exact
    }
    default {
        Write-Host "🔍 Running individual CSV test cases (default)..." -ForegroundColor Green
        cargo test test_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
    }
}

Write-Host ""
Write-Host "💡 To modify test data:" -ForegroundColor Cyan
Write-Host "   Edit: $csvFile" -ForegroundColor White
Write-Host "   Then re-run this script to test your changes" -ForegroundColor White
Write-Host ""
Write-Host "✨ Test complete!" -ForegroundColor Green
