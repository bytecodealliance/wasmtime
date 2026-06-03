#!/bin/bash

set -exuo pipefail

script_directory=$(dirname -- "${BASH_SOURCE[0]}")
working_directory=$(mktemp -d)

# Tests.
tests_directory="${working_directory}/tests"
mkdir -p "${tests_directory}"

"${script_directory}/tests.sh" "${tests_directory}"

# Collect traces.
traces_directory="${working_directory}/traces"
mkdir -p "${traces_directory}"

"${script_directory}/collect.sh" "${tests_directory}" "${traces_directory}"

trace_file="${working_directory}/trace.txt"
cat "${traces_directory}/"* > "${trace_file}"

# Report.
report_file="${working_directory}/report.txt"
"${script_directory}/report.py" "$@" < "${trace_file}" > "${report_file}"

# Wrap.
cat "${report_file}"
echo "${working_directory}"
