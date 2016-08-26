#!/bin/bash

# Check Python sources for Python 3 compatibility using pylint.
#
# Install pylint with 'pip install pylint'.
cd $(dirname "$0")
pylint --py3k --reports=no -- *.py cretonne isa
flake8 .
