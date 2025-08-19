@echo off
echo.
echo ========================================
echo   UWH RefBox Font Sizing Test Runner
echo ========================================
echo.

echo 1. CSV-based font sizing test (individual test cases)
echo 2. Group font sizing test (multiple referee names)
echo 3. Demo test (your specification data)
echo 4. Manual testing instructions
echo 5. Run all tests
echo.

set /p choice="Enter your choice (1-5): "

echo.
echo You entered: %choice%
echo.

if "%choice%"=="1" goto choice1
if "%choice%"=="2" goto choice2
if "%choice%"=="3" goto choice3
if "%choice%"=="4" goto choice4
if "%choice%"=="5" goto choice5
goto default

:choice1
echo Running CSV-based font sizing test...
cargo test test_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
goto end

:choice2
echo Running group font sizing test...
cargo test test_group_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
goto end

:choice3
echo Running demo test with your specification data...
cargo test test_csv_based_font_sizing_demo --package refbox --bin refbox -- --nocapture --exact
goto end

:choice4
echo Showing manual testing instructions...
cargo test test_manual_csv_update_instructions --package refbox --bin refbox -- --nocapture --exact
goto end

:choice5
echo Running all font sizing tests...
echo.
echo [1/4] CSV data test...
cargo test test_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
echo.
echo [2/4] Group sizing test...
cargo test test_group_font_sizing_with_csv_data --package refbox --bin refbox -- --nocapture --exact
echo.
echo [3/4] Demo test...
cargo test test_csv_based_font_sizing_demo --package refbox --bin refbox -- --nocapture --exact
echo.
echo [4/4] Instructions...
cargo test test_manual_csv_update_instructions --package refbox --bin refbox -- --nocapture --exact
goto end

:default
echo Invalid choice "%choice%". Running demo test (default)...
echo Please enter only numbers 1-5 next time.
echo.
cargo test test_csv_based_font_sizing_demo --package refbox --bin refbox -- --nocapture --exact

:end

echo.
echo ========================================
echo To modify test data:
echo   Edit: refbox/test_data/font_sizing_test_data.csv
echo   Then re-run this script
echo ========================================
echo.
pause
