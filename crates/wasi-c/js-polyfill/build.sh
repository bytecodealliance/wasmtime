#!/bin/bash
set -euo pipefail

EMCC=emcc

# TODO: Remove the clang include once Emscripten supports <stdatomic.h>

"$EMCC" ../sandboxed-system-primitives/src/*.c \
    -DWASMTIME_SSP_WASI_API \
    -DWASMTIME_SSP_STATIC_CURFDS \
    -I../sandboxed-system-primitives/include \
    -Iclang \
    --shell-file shell.html \
    polyfill.c \
    -s WARN_ON_UNDEFINED_SYMBOLS=0 \
    -s EXPORTED_FUNCTIONS="['_main', '_handleFiles', '___wasi_args_get', '___wasi_args_sizes_get', '___wasi_clock_res_get', '___wasi_clock_time_get', '___wasi_environ_get', '___wasi_environ_sizes_get', '___wasi_fd_prestat_get', '___wasi_fd_prestat_dir_name', '___wasi_fd_close', '___wasi_fd_datasync', '___wasi_fd_pread', '___wasi_fd_pwrite', '___wasi_fd_read', '___wasi_fd_renumber', '___wasi_fd_seek', '___wasi_fd_tell', '___wasi_fd_fdstat_get', '___wasi_fd_fdstat_set_flags', '___wasi_fd_fdstat_set_rights', '___wasi_fd_sync', '___wasi_fd_write', '___wasi_fd_advise', '___wasi_fd_allocate', '___wasi_path_create_directory', '___wasi_path_link', '___wasi_path_open', '___wasi_fd_readdir', '___wasi_path_readlink', '___wasi_path_rename', '___wasi_fd_filestat_get', '___wasi_fd_filestat_set_times', '___wasi_fd_filestat_set_size', '___wasi_path_filestat_get', '___wasi_path_filestat_set_times', '___wasi_path_symlink', '___wasi_path_unlink_file', '___wasi_path_remove_directory', '___wasi_poll_oneoff', '___wasi_proc_exit', '___wasi_proc_raise', '___wasi_random_get', '___wasi_sched_yield', '___wasi_sock_recv', '___wasi_sock_send', '___wasi_sock_shutdown']" \
    --pre-js wasi.js \
    -o polyfill.html
