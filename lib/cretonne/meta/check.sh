#!/bin/bash
set -e
cd $(dirname "$0")

echo "=== Python unit tests ==="
python -m unittest discover

runif() {
    if command -v "$1" > /dev/null; then
        echo "=== $1 ==="
        "$@"
    else
        echo "$1 not found"
    fi
}

# Check Python sources for Python 3 compatibility using pylint.
#
# Install pylint with 'pip install pylint'.
runif pylint --py3k --reports=no -- *.py cretonne isa
runif flake8 .
runif mypy --py2 build.py
