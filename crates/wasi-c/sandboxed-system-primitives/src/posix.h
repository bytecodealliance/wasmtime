// Part of the Wasmtime Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://github.com/bytecodealliance/wasmtime/blob/master/LICENSE for license information.
//
// Significant parts of this file are derived from cloudabi-utils. See
// https://github.com/bytecodealliance/wasmtime/blob/master/lib/wasi/sandboxed-system-primitives/src/LICENSE
// for license information.
//
// The upstream file contains the following copyright notice:
//
// Copyright (c) 2016-2018 Nuxi, https://nuxi.nl/

#ifndef POSIX_H
#define POSIX_H

#include <stdbool.h>
#include <stddef.h>

#include "locking.h"

struct fd_entry;
struct fd_prestat;
struct syscalls;

struct fd_table {
  struct rwlock lock;
  struct fd_entry *entries;
  size_t size;
  size_t used;
};

struct fd_prestats {
  struct rwlock lock;
  struct fd_prestat *prestats;
  size_t size;
  size_t used;
};

struct argv_environ_values {
  size_t argc;
  size_t argv_buf_size;
  char **argv;
  char *argv_buf;
  size_t environ_count;
  size_t environ_buf_size;
  char **environ;
  char *environ_buf;
};

void fd_table_init(struct fd_table *);
bool fd_table_insert_existing(struct fd_table *, __wasi_fd_t, int);
void fd_prestats_init(struct fd_prestats *);
bool fd_prestats_insert(struct fd_prestats *, const char *, __wasi_fd_t);
void argv_environ_init(struct argv_environ_values *,
                       const size_t *argv_offsets, size_t argv_offsets_len,
                       const char *argv_buf, size_t argv_buf_len,
                       const size_t *environ_offsets, size_t environ_offsets_len,
                       const char *environ_buf, size_t environ_buf_len);

#endif
