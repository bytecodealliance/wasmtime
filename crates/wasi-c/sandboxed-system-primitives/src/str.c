// Part of the Wasmtime Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://github.com/bytecodealliance/wasmtime/blob/master/LICENSE for license information.
//
// Significant parts of this file are derived from cloudabi-utils. See
// https://github.com/bytecodealliance/wasmtime/blob/master/lib/wasi/sandboxed-system-primitives/src/LICENSE
// for license information.
//
// The upstream file contains the following copyright notice:
//
// Copyright (c) 2016 Nuxi, https://nuxi.nl/

#include "config.h"

#include <errno.h>
#include <stdlib.h>
#include <string.h>

#include "str.h"

char *str_nullterminate(const char *s, size_t len) {
  // Copy string.
  char *ret = strndup(s, len);
  if (ret == NULL)
    return NULL;

  // Ensure that it contains no null bytes within.
  if (strlen(ret) != len) {
    free(ret);
    errno = EILSEQ;
    return NULL;
  }
  return ret;
}
