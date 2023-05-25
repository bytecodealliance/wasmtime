#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    -p test-programs \
    $@
