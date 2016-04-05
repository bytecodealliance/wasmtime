# Second-level build script.
#
# This script is run from src/libcretonne/build.rs to generate Rust files.

import argparse

parser = argparse.ArgumentParser(description='Generate sources for Cretonne.')
parser.add_argument('--out-dir', help='set output directory')

args = parser.parse_args()
out_dir = args.out_dir
