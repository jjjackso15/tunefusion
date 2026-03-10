#!/bin/bash
# Test script to debug Demucs detection
# Run this on the machine where Demucs is installed

echo "=== Environment Info ==="
echo "HOME: $HOME"
echo "PATH: $PATH"
echo "PYTHONPATH: $PYTHONPATH"
echo ""

echo "=== Python Version ==="
python3 --version
echo ""

echo "=== Looking for site-packages ==="
for version in 3.14 3.13 3.12 3.11 3.10 3.9; do
    dir="$HOME/.local/lib/python$version/site-packages"
    if [ -d "$dir" ]; then
        echo "FOUND: $dir"
        if [ -d "$dir/demucs" ]; then
            echo "  -> demucs is HERE"
        fi
    fi
done
echo ""

echo "=== Testing import without PYTHONPATH ==="
python3 -c "import demucs; print('SUCCESS:', demucs.__file__)" 2>&1
echo ""

echo "=== Testing with explicit PYTHONPATH ==="
export PYTHONPATH="$HOME/.local/lib/python3.14/site-packages:$PYTHONPATH"
echo "PYTHONPATH now: $PYTHONPATH"
python3 -c "import demucs; print('SUCCESS:', demucs.__file__)" 2>&1
echo ""

echo "=== Testing demucs CLI ==="
which demucs 2>/dev/null || echo "demucs not in PATH"
$HOME/.local/bin/demucs --help 2>&1 | head -3 || echo "demucs CLI not found in ~/.local/bin"
