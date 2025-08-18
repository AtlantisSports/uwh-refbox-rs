@echo off
echo.
echo ========================================
echo   UWH RefBox Font Sizing Demo Mode
echo ========================================
echo.

echo This will run the RefBox application with test data populated
echo to demonstrate the dynamic font sizing functionality.
echo.
echo You will see the GUI with referee names populated in the cells.
echo Look at the referee information section to see font sizing in action.
echo.

echo Available demo modes:
echo 1. Specification data (long referee names from your requirements)
echo 2. Short names (should not trigger font reduction)
echo.

set /p choice="Enter your choice (1-2): "

if "%choice%"=="1" (
    echo.
    echo Starting RefBox with specification test data...
    echo You should see:
    echo - Russell Owen Camilo La Torre (Chief Ref)
    echo - Norfatin Aainaa Binti Hashim (Timer)
    echo - Tuan San Jonathan Chan (Water Ref 1)
    echo - Muhammad Danish Haikal Mohd Fadel (Water Ref 2)
    echo - A very long person name (Water Ref 3)
    echo.
    echo Font sizes should be reduced to fit the long names.
    echo.
    cargo run --bin refbox -- --font-demo --demo-data specification --no-simulate
) else if "%choice%"=="2" (
    echo.
    echo Starting RefBox with short name test data...
    echo You should see:
    echo - John Smith (Chief Ref)
    echo - Jane Doe (Timer)
    echo - Bob Wilson (Water Ref 1)
    echo - Sue Chen (Water Ref 2)
    echo - Tom Brown (Water Ref 3)
    echo.
    echo Font sizes should remain at default size.
    echo.
    cargo run --bin refbox -- --font-demo --demo-data short --no-simulate
) else (
    echo.
    echo Starting RefBox with specification test data (default)...
    cargo run --bin refbox -- --font-demo --demo-data specification --no-simulate
)

echo.
echo ========================================
echo Demo completed!
echo.
echo To run unit tests instead:
echo   run_font_tests.bat
echo.
echo To modify test data:
echo   Edit: refbox/test_data/font_sizing_test_data.csv
echo ========================================
echo.
pause
