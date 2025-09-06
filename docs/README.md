# Documentation Directory

This directory contains all documentation and documentation-related scripts for the UWH RefBox Rust project.

## Structure

### `design/`
Contains design documents and specifications:
- `Atlantis UWH-REFBOX-RS Detailed Design.md` - Main design document (Markdown source)
- `Atlantis UWH-REFBOX-RS Detailed Design.html` - Generated HTML version

### `scripts/`
Contains scripts for documentation generation and maintenance:
- `generate_html.py` - Converts the Markdown design document to HTML with custom styling

## Usage

### Generating HTML Documentation

To regenerate the HTML version of the design document:

```bash
cd docs/scripts
python generate_html.py
```

This will:
1. Read the Markdown file from `../design/`
2. Convert it to HTML with custom dark theme styling
3. Generate a responsive layout with navigation
4. Save the result back to `../design/`

### Viewing Documentation

Open `design/Atlantis UWH-REFBOX-RS Detailed Design.html` in any web browser to view the formatted documentation with:
- Dark theme with blue accents
- Grid-based table of contents
- Responsive design for mobile and desktop
- Proper syntax highlighting for code blocks
- Nested list indentation

## Maintenance

When updating the design document:
1. Edit the Markdown file: `design/Atlantis UWH-REFBOX-RS Detailed Design.md`
2. Regenerate HTML: `cd scripts && python generate_html.py`
3. Commit both files to preserve the HTML version in the repository
