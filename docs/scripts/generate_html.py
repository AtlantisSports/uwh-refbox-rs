#!/usr/bin/env python3
"""
Generate HTML version of the Atlantis UWH-REFBOX-RS Detailed Design document
"""

import re
import html

def markdown_to_html(markdown_content):
    """Convert markdown to HTML with dark theme and navigation"""

    # Extract sections for navigation
    sections = []
    lines = markdown_content.split('\n')
    for line in lines:
        if line.startswith('## '):
            title = line[3:].strip()
            # Create ID by keeping numbers and converting to lowercase with dashes
            section_id = title.lower().replace(' ', '-').replace('.', '').replace('(', '').replace(')', '').replace(',', '').replace('/', '-')
            sections.append((section_id, title))

    # Start with dark theme HTML structure
    html_content = """<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>🏗️ Atlantis UWH-REFBOX-RS Detailed Design</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #1e3c72 0%, #2a5298 100%);
            color: #e8eaed;
            line-height: 1.6;
            min-height: 100vh;
        }

        .header {
            background: rgba(0, 0, 0, 0.3);
            padding: 2rem 0;
            text-align: center;
            border-bottom: 2px solid rgba(255, 255, 255, 0.1);
        }

        .header h1 {
            color: #ffffff;
            font-size: 2.5rem;
            margin-bottom: 0.5rem;
            text-shadow: 2px 2px 4px rgba(0, 0, 0, 0.5);
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 1rem;
        }

        .header .logo {
            width: 60px;
            height: 60px;
            object-fit: contain;
        }

        .header .subtitle {
            color: #b8c5d6;
            font-size: 1.1rem;
            font-weight: 300;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }

        .toc-section {
            background: rgba(0, 0, 0, 0.4);
            border-radius: 12px;
            padding: 2rem;
            margin-bottom: 2rem;
            border: 1px solid rgba(255, 255, 255, 0.1);
        }

        .toc-section h2 {
            color: #4a90e2;
            font-size: 1.5rem;
            margin-bottom: 1.5rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .toc-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
            gap: 1rem;
            margin-bottom: 2rem;
        }

        .toc-item {
            background: rgba(0, 0, 0, 0.3);
            border: 1px solid rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            padding: 1rem;
            transition: all 0.3s ease;
            text-decoration: none;
            color: #4a90e2;
            display: block;
            font-weight: 500;
        }

        .toc-item:hover {
            background: rgba(74, 144, 226, 0.1);
            border-color: #4a90e2;
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(74, 144, 226, 0.2);
        }

        .main-content {
            background: rgba(0, 0, 0, 0.3);
            border-radius: 12px;
            padding: 2rem;
            border: 1px solid rgba(255, 255, 255, 0.1);
        }

        .section {
            margin-bottom: 3rem;
            padding-bottom: 2rem;
            border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        }

        .section:last-child {
            border-bottom: none;
        }

        .return-btn {
            display: inline-block;
            background: linear-gradient(135deg, #4a90e2, #357abd);
            color: white;
            padding: 0.5rem 1rem;
            border-radius: 6px;
            text-decoration: none;
            font-size: 0.9rem;
            margin-bottom: 1rem;
            transition: all 0.3s ease;
            border: 1px solid rgba(255, 255, 255, 0.2);
        }

        .return-btn:hover {
            background: linear-gradient(135deg, #357abd, #2968a3);
            transform: translateY(-2px);
            box-shadow: 0 4px 12px rgba(74, 144, 226, 0.3);
        }

        h1 {
            color: #ffffff;
            font-size: 2.2rem;
            margin-bottom: 1rem;
            text-shadow: 1px 1px 2px rgba(0, 0, 0, 0.5);
        }

        h2 {
            color: #4a90e2;
            font-size: 1.8rem;
            margin: 2rem 0 1rem 0;
            border-bottom: 2px solid #4a90e2;
            padding-bottom: 0.5rem;
        }

        h3 {
            color: #6bb6ff;
            font-size: 1.4rem;
            margin: 1.5rem 0 0.75rem 0;
        }

        h4 {
            color: #8cc8ff;
            font-size: 1.2rem;
            margin: 1rem 0 0.5rem 0;
        }

        p {
            margin-bottom: 1rem;
            color: #e8eaed;
        }

        ul, ol {
            margin: 1rem 0;
            padding-left: 2rem;
        }

        li {
            margin: 0.5rem 0;
            color: #e8eaed;
            line-height: 1.6;
        }

        /* Nested list indentation */
        ul ul, ol ol, ul ol, ol ul {
            margin: 0.25rem 0;
            padding-left: 2rem;
        }

        ul ul ul, ol ol ol {
            padding-left: 2rem;
        }

        .tree-diagram {
            background: rgba(0, 0, 0, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            padding: 1.5rem;
            margin: 1.5rem 0;
            font-family: 'Courier New', monospace;
            font-size: 0.9rem;
            line-height: 1.4;
            color: #e8eaed;
            overflow-x: auto;
        }

        .tree-diagram .comment {
            color: #7dd3fc;
        }

        .tree-diagram .folder {
            color: #fbbf24;
            font-weight: bold;
        }

        .tree-diagram .file {
            color: #a7f3d0;
        }

        code {
            background: rgba(0, 0, 0, 0.4);
            color: #fbbf24;
            padding: 0.2rem 0.4rem;
            border-radius: 4px;
            font-family: 'Courier New', monospace;
            font-size: 0.9rem;
        }

        pre {
            background: rgba(0, 0, 0, 0.6);
            border: 1px solid rgba(255, 255, 255, 0.2);
            border-radius: 8px;
            padding: 1.5rem;
            margin: 1.5rem 0;
            overflow-x: auto;
            color: #e8eaed;
        }

        pre code {
            background: none;
            padding: 0;
            color: inherit;
        }

        .status-badge {
            display: inline-block;
            padding: 0.25rem 0.75rem;
            border-radius: 12px;
            font-size: 0.8rem;
            font-weight: bold;
            margin-left: 0.5rem;
        }

        .status-planned {
            background: #f59e0b;
            color: #000;
        }

        .status-complete {
            background: #10b981;
            color: #000;
        }

        .timer-highlight {
            background: linear-gradient(135deg, rgba(74, 144, 226, 0.2), rgba(53, 122, 189, 0.2));
            border: 1px solid rgba(74, 144, 226, 0.4);
            border-radius: 8px;
            padding: 1.5rem;
            margin: 1.5rem 0;
        }

        @media (max-width: 768px) {
            .container {
                padding: 1rem;
            }

            .toc-grid {
                grid-template-columns: 1fr;
            }

            .header h1 {
                font-size: 2rem;
            }
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>
            <img src="[PNG] (RGB) - Atlantis Sports - Lambda_Original.png" alt="Atlantis Sports Logo" class="logo">
            Atlantis UWH-REFBOX-RS Detailed Design
        </h1>
        <p class="subtitle">Comprehensive design documentation including application architecture, testing strategy, and system performance requirements</p>
    </div>

    <div class="container">
        <div class="toc-section" id="index">
            <h2>📋 Table of Contents</h2>
            <div class="toc-grid">"""

    # Add navigation links in grid format
    for section_id, title in sections:
        html_content += f'\n                <a href="#{section_id}" class="toc-item">{title}</a>'

    html_content += """
            </div>
        </div>

        <main class="main-content">"""
    
    # Process the markdown content
    lines = markdown_content.split('\n')
    in_code_block = False
    in_list = False
    current_section = None

    for i, line in enumerate(lines):
        line = line.rstrip()

        # Handle code blocks
        if line.startswith('```'):
            if in_code_block:
                html_content += '</code></pre>\n'
                in_code_block = False
            else:
                lang = line[3:].strip()
                html_content += f'<pre><code class="language-{lang}">\n'
                in_code_block = True
            continue

        if in_code_block:
            html_content += html.escape(line) + '\n'
            continue

        # Handle headers
        if line.startswith('# '):
            if in_list:
                html_content += '</ul>\n'
                in_list = False
            html_content += f'<h1>{html.escape(line[2:])}</h1>\n'
        elif line.startswith('## '):
            if in_code_block:
                html_content += '</div>\n'
                in_code_block = False
            if in_list:
                html_content += '</ul>\n'
                in_list = False
            # Close previous section
            if current_section:
                html_content += '</div>\n'

            # Start new section
            title = line[3:].strip()
            section_id = title.lower().replace(' ', '-').replace('.', '').replace('(', '').replace(')', '').replace(',', '').replace('/', '-')
            current_section = section_id



            # Add return button and section wrapper
            html_content += f'<div class="section" id="{section_id}">\n'
            html_content += '<a href="#index" class="return-btn">↑ Return to Index</a>\n'

            # Special styling for timer section
            if 'Timer System' in title:
                html_content += f'<div class="timer-highlight">\n<h2>{html.escape(title)}</h2>\n'
            else:
                html_content += f'<h2>{html.escape(title)}</h2>\n'

        elif line.startswith('### '):
            if in_list:
                html_content += '</ul>\n'
                in_list = False
            html_content += f'<h3>{html.escape(line[4:])}</h3>\n'
        elif line.startswith('#### '):
            if in_list:
                html_content += '</ul>\n'
                in_list = False
            html_content += f'<h4>{html.escape(line[5:])}</h4>\n'

        # Handle lists (including deeply nested)
        elif re.match(r'^(\s*- )', line):
            # Regular list item - detect any amount of indentation
            if in_code_block:
                html_content += '</div>\n'
                in_code_block = False

            # Count leading spaces before the dash
            spaces = len(line) - len(line.lstrip())
            dash_pos = line.find('- ')
            content = line[dash_pos + 2:]

            # Calculate indentation level (every 2 spaces = 1 level)
            indent_level = spaces // 2

            # Handle nested lists
            if not in_list:
                html_content += '<ul>\n'
                in_list = True

            # Handle bold text
            content = re.sub(r'\*\*(.*?)\*\*', r'<strong>\1</strong>', content)
            # Handle inline code
            content = re.sub(r'`([^`]+)`', r'<code>\1</code>', content)
            # Handle status badges
            if 'PLANNED' in content:
                content = content.replace('PLANNED', '<span class="status-badge status-planned">PLANNED</span>')

            # Add indentation styling based on level
            if indent_level > 0:
                margin = indent_level * 1.5
                html_content += f'<li style="margin-left: {margin}rem;">{content}</li>\n'
            else:
                html_content += f'<li>{content}</li>\n'

        # Handle tree diagrams (only lines with actual tree characters)
        elif '├──' in line or '└──' in line or '│' in line:

            # This is a tree diagram
            if not in_code_block:
                html_content += '<div class="tree-diagram">\n'
                in_code_block = True

            # Format tree line with colors
            formatted_line = html.escape(line)
            # Color comments (text after #)
            if '#' in formatted_line:
                parts = formatted_line.split('#', 1)
                if len(parts) == 2:
                    formatted_line = parts[0] + '<span class="comment"># ' + parts[1] + '</span>'

            # Color folders and files
            if '/' in formatted_line and not formatted_line.strip().endswith('/'):
                formatted_line = re.sub(r'(\w+/)(?!</span>)', r'<span class="folder">\1</span>', formatted_line)
            elif formatted_line.strip().endswith('/'):
                formatted_line = re.sub(r'(\w+/)$', r'<span class="folder">\1</span>', formatted_line)
            elif any(ext in formatted_line for ext in ['.rs', '.toml', '.md', '.py']):
                formatted_line = re.sub(r'(\w+\.\w+)', r'<span class="file">\1</span>', formatted_line)

            html_content += formatted_line + '\n'
            continue

        # Handle empty lines
        elif line.strip() == '':
            if in_code_block:
                html_content += '</div>\n'
                in_code_block = False
            if in_list:
                html_content += '</ul>\n'
                in_list = False
            html_content += '\n'

        # Handle regular paragraphs
        else:
            if in_code_block:
                html_content += '</div>\n'
                in_code_block = False
            if in_list:
                html_content += '</ul>\n'
                in_list = False

            if not line.startswith(('Author:', 'Date:', 'Last Updated:')):
                content = html.escape(line)
                # Handle bold text
                content = re.sub(r'\*\*(.*?)\*\*', r'<strong>\1</strong>', content)
                # Handle inline code
                content = re.sub(r'`([^`]+)`', r'<code>\1</code>', content)
                # Handle status indicators
                content = re.sub(r'✅', '<span class="status-complete">✅</span>', content)
                html_content += f'<p>{content}</p>\n'
    
    # Close any open elements
    if in_list:
        html_content += '</ul>\n'
    if in_code_block and 'tree-diagram' in html_content:
        html_content += '</div>\n'
    if current_section:
        # Close timer highlight if it was opened
        if 'Timer System' in html_content and 'timer-highlight' in html_content:
            html_content += '</div>\n'
        html_content += '</div>\n'  # Close section

    # Close HTML structure
    html_content += """
        </main>
    </div>
</body>
</html>"""
    
    return html_content

def main():
    # Read the markdown file from the design directory
    markdown_path = '../design/Atlantis UWH-REFBOX-RS Detailed Design.md'
    html_path = '../design/Atlantis UWH-REFBOX-RS Detailed Design.html'

    with open(markdown_path, 'r', encoding='utf-8') as f:
        markdown_content = f.read()

    # Convert to HTML
    html_content = markdown_to_html(markdown_content)

    # Write HTML file to the design directory
    with open(html_path, 'w', encoding='utf-8') as f:
        f.write(html_content)

    print("✅ HTML document generated successfully!")
    print(f"📄 File: {html_path}")

if __name__ == '__main__':
    main()
