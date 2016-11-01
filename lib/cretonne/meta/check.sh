#!/bin/bash
set -e
cd $(dirname "$0")

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

# Style linting.
runif flake8 .

# Type checking.
runif mypy --py2 build.py

echo "=== Python unit tests ==="
python -m unittest discover

# Then run the unit tests again with Python 3.
# We get deprecation warnings about assertRaisesRegexp which was renamed in
# Python 3, but there doesn't seem to be an easy workaround.
runif python3 -Wignore:Deprecation -m unittest discover

