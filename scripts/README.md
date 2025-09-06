# Development Workflow and Scripts

This directory contains scripts to automate quality checks and commits for the uwh-refbox-rs project.

## Quick Start

### Option 1: Use the Smart Commit Script (Recommended)
```powershell
# Run quality checks and commit in one command
.\scripts\commit.ps1 "Your commit message here"
```

### Option 2: Use VS Code Tasks
- Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on Mac)
- Type "Tasks: Run Task"
- Select "Smart commit" to commit with automatic quality checks
- Or select "Pre-commit checks" to just run the quality checks

### Option 3: Manual Quality Checks
```powershell
# Run all quality checks manually
.\scripts\pre-commit-checks.ps1

# Then commit normally
git add .
git commit -m "Your commit message"
```

## Available Scripts

### `pre-commit-checks.ps1`
Runs the complete quality check suite:
1. **Code Formatting**: `cargo fmt --check` (auto-fixes if needed)
2. **Linting**: `cargo clippy` (checks for common mistakes and improvements)
3. **Build**: `cargo build` (ensures code compiles)
4. **Testing**: `cargo test` (runs all tests)

### `commit.ps1`
Smart commit script that:
1. Runs all quality checks first
2. Stages all changes (`git add .`)
3. Commits with your message
4. Provides clear success/failure feedback

#### Usage:
```powershell
.\scripts\commit.ps1 "feat: add new language button layout"
.\scripts\commit.ps1 "fix: resolve formatting issues in configuration.rs"
.\scripts\commit.ps1 "refactor: reorganize user options layout" -SkipChecks  # Not recommended
```

## Git Hooks

A pre-commit hook has been installed at `.git/hooks/pre-commit` that automatically runs quality checks before each commit. If any check fails, the commit will be aborted.

To bypass the hook (not recommended):
```bash
git commit -m "your message" --no-verify
```

## VS Code Integration

The following VS Code tasks are available:

- **Pre-commit checks**: Run quality checks without committing
- **Smart commit**: Interactive commit with automatic quality checks
- **Format code**: Run `cargo fmt` only
- **Clippy check**: Run `cargo clippy` only

Access via: `Ctrl+Shift+P` → "Tasks: Run Task"

## Quality Standards

All commits must pass:
- ✅ **Formatting**: Code must be formatted with `rustfmt`
- ✅ **Linting**: No clippy warnings or errors
- ✅ **Compilation**: Code must build successfully
- ✅ **Testing**: All tests must pass

## Troubleshooting

### PowerShell Execution Policy Error
If you get execution policy errors, run:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Script Not Found
Make sure you're running scripts from the project root directory:
```powershell
cd "C:\Copilot Projects\uwh-refbox-rs"
.\scripts\commit.ps1 "your message"
```

### Quality Check Failures
The scripts will show exactly what failed and provide guidance on how to fix issues. Common solutions:
- **Formatting**: The script auto-runs `cargo fmt` to fix formatting
- **Clippy**: Review and fix the reported issues
- **Build**: Fix compilation errors shown in the output
- **Tests**: Fix failing tests or update them if behavior changed intentionally
