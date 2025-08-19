# Integration Test Scripts

This directory contains scripts for running integration tests and demonstrations related to the UWH RefBox application.

## Scripts

### Font Testing Scripts

#### `run_font_demo.bat` (Windows)
Runs the font sizing demonstration to show dynamic font scaling in action.

**Usage:**
```cmd
run_font_demo.bat
```

#### `run_font_tests.bat` (Windows)
Executes the font sizing test suite to validate font scaling logic.

**Usage:**
```cmd
run_font_tests.bat
```

#### `test_font_sizing.ps1` (PowerShell)
PowerShell script for comprehensive font sizing tests with detailed output.

**Usage:**
```powershell
.\test_font_sizing.ps1
```

## Running Tests

### From Project Root
You can run these scripts from the project root directory:

```bash
# Windows Command Prompt
integration-tests\scripts\run_font_demo.bat
integration-tests\scripts\run_font_tests.bat

# PowerShell
.\integration-tests\scripts\test_font_sizing.ps1
```

### From Scripts Directory
Or navigate to the scripts directory first:

```bash
cd integration-tests/scripts

# Then run any script
./run_font_demo.bat
./run_font_tests.bat
./test_font_sizing.ps1
```

## Test Coverage

These scripts test:
- Dynamic font sizing algorithms
- UI layout responsiveness
- Text overflow handling
- Multi-language text rendering
- Referee name display formatting

## Adding New Scripts

When adding new test scripts:
1. Place them in this directory
2. Update this README with usage instructions
3. Ensure scripts work from both project root and scripts directory
4. Add appropriate error handling and output formatting
