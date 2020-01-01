#!/bin/bash
set -euo pipefail

WORKDIR=assets                      # our workdir
JS_LIBRARY=$WORKDIR/load-files.js   # JS helper lib which we use in main.rs to handle loading WASI binaries in Emscripten
EM_SHELL=$WORKDIR/shell.html        # basic Emscripten shell env
WASI=$WORKDIR/wasi.js               # WASI polyfill JS glue code
POLYFILL=$WORKDIR/polyfill.html     # our output

EXPORTED_FUNCTIONS=(
  _main
  _get_wasi_context
  _handleFiles
  _old_wasi_common_args_get
  _old_wasi_common_args_sizes_get
  _old_wasi_common_clock_res_get
  _old_wasi_common_clock_time_get
  _old_wasi_common_environ_get
  _old_wasi_common_environ_sizes_get
  _old_wasi_common_fd_advise
  _old_wasi_common_fd_allocate
  _old_wasi_common_fd_close
  _old_wasi_common_fd_datasync
  _old_wasi_common_fd_fdstat_get
  _old_wasi_common_fd_fdstat_set_flags
  _old_wasi_common_fd_fdstat_set_rights
  _old_wasi_common_fd_filestat_get
  _old_wasi_common_fd_filestat_set_size
  _old_wasi_common_fd_filestat_set_times
  _old_wasi_common_fd_pread
  _old_wasi_common_fd_prestat_dir_name
  _old_wasi_common_fd_prestat_get
  _old_wasi_common_fd_pwrite
  _old_wasi_common_fd_read
  _old_wasi_common_fd_readdir
  _old_wasi_common_fd_renumber
  _old_wasi_common_fd_seek
  _old_wasi_common_fd_sync
  _old_wasi_common_fd_tell
  _old_wasi_common_fd_write
  _old_wasi_common_path_create_directory
  _old_wasi_common_path_filestat_get
  _old_wasi_common_path_filestat_set_times
  _old_wasi_common_path_link
  _old_wasi_common_path_open
  _old_wasi_common_path_readlink
  _old_wasi_common_path_remove_directory
  _old_wasi_common_path_rename
  _old_wasi_common_path_symlink
  _old_wasi_common_path_unlink_file
  _old_wasi_common_poll_oneoff
  _old_wasi_common_proc_exit
  _old_wasi_common_proc_raise
  _old_wasi_common_random_get
  _old_wasi_common_sched_yield
  _old_wasi_common_sock_recv
  _old_wasi_common_sock_send
  _old_wasi_common_sock_shutdown
)
EXPORTED_FUNCTIONS_CONCAT=$(printf ",'%s'" "${EXPORTED_FUNCTIONS[@]}")
EXPORTED_FUNCTIONS_CONCAT=${EXPORTED_FUNCTIONS_CONCAT:1}

cargo +nightly rustc --target wasm32-unknown-emscripten --release -vv -- -C link-args="--js-library ${JS_LIBRARY} --shell-file ${EM_SHELL} --pre-js ${WASI} -s EXPORTED_FUNCTIONS=[${EXPORTED_FUNCTIONS_CONCAT}] -o ${POLYFILL}"
