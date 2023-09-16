#!/bin/bash

cargo test \
    --features "test-programs/test_programs" \
    --package test-programs \
    $@
