#!/bin/bash
set -e
cd $(dirname "$0")

# Run unit tests.
python -m unittest discover

# Check Python sources for Python 3 compatibility using pylint.
#
# Install pylint with 'pip install pylint'.
pylint --py3k --reports=no -- *.py cretonne isa
flake8 .
