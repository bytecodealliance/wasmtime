#!/bin/bash
set -euo pipefail
cd $(dirname "$0")

function runif {
    if command -v "$1" > /dev/null; then
        echo "   === $1 ==="
        "$@"
    else
        echo "$1 not found"
    fi
}

# Style linting.
runif flake8 .

# Type checking.
runif mypy --py2 build.py

# Python unit tests.
runif python -m unittest discover

# Then run the unit tests again with Python 3.
# We get deprecation warnings about assertRaisesRegexp which was renamed in
# Python 3, but there doesn't seem to be an easy workaround.
runif python3 -Wignore:Deprecation -m unittest discover
