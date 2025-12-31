#!/bin/bash
# Convert BEAM 2.0 Presentation to various formats

echo "Converting BEAM 2.0: Silica as Backend for Functional Languages..."

# Check if marp is available
if command -v marp &> /dev/null; then
    echo "Using Marp to generate PDF..."
    marp *.md --pdf --output beam2.0_presentation.pdf
    echo "Generated: beam2.0_presentation.pdf"
else
    echo "Marp not found. Install with: npm install -g @marp-team/marp-cli"
fi

# Create HTML version with reveal.js structure
echo "Creating HTML presentation structure..."

cat > presentation.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>BEAM 2.0: Silica as Backend for Functional Languages</title>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.5.0/reveal.min.css">
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.5.0/theme/black.min.css">
    <style>
        .reveal h1 { color: #00ADD8; }
        .reveal h2 { color: #00ADD8; }
        .reveal code { background: #2a2a2a; color: #00ADD8; }
        .reveal pre { background: #1a1a1a; }
    </style>
</head>
<body>
    <div class="reveal">
        <div class="slides">
EOF

# Convert each markdown file to HTML slides
for file in [0-9][0-9]_*.md; do
    if [ -f "$file" ]; then
        echo "Processing $file..."
        echo "<section data-markdown=\"$file\" data-separator=\"---\"></section>" >> presentation.html
    fi
done

cat >> presentation.html << 'EOF'
        </div>
    </div>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.5.0/reveal.min.js"></script>
    <script src="https://cdnjs.cloudflare.com/ajax/libs/reveal.js/4.5.0/plugin/markdown/markdown.min.js"></script>
    <script>
        Reveal.initialize({
            plugins: [RevealMarkdown],
            hash: true,
            slideNumber: true,
            transition: 'slide',
            backgroundTransition: 'slide',
            overview: true,
            center: true,
            width: 1280,
            height: 720
        });
    </script>
</body>
</html>
EOF

echo "Generated: presentation.html"
echo "Open presentation.html in a browser to view the slides."
echo ""
echo "Conversion complete!"
