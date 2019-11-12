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

#ifndef CONFIG_H
#define CONFIG_H

#include <stdlib.h>

#if defined(__FreeBSD__) || defined(__APPLE__)
#define CONFIG_HAS_ARC4RANDOM_BUF 1
#else
#define CONFIG_HAS_ARC4RANDOM_BUF 0
#endif

// On Linux, prefer to use getrandom, though it isn't available in
// GLIBC before 2.25.
#if defined(__linux__) && \
    (!defined(__GLIBC__) || \
     __GLIBC__ > 2 || \
     (__GLIBC__ == 2 && __GLIBC_MINOR__ >= 25))
#define CONFIG_HAS_GETRANDOM 1
#else
#define CONFIG_HAS_GETRANDOM 0
#endif

#if defined(__CloudABI__)
#define CONFIG_HAS_CAP_ENTER 1
#else
#define CONFIG_HAS_CAP_ENTER 0
#endif

#if !defined(__APPLE__) && !defined(__FreeBSD__) && !defined(__EMSCRIPTEN__)
#define CONFIG_HAS_CLOCK_NANOSLEEP 1
#else
#define CONFIG_HAS_CLOCK_NANOSLEEP 0
#endif

#if !defined(__APPLE__) && !defined(__FreeBSD__)
#define CONFIG_HAS_FDATASYNC 1
#else
#define CONFIG_HAS_FDATASYNC 0
#endif

#ifndef __CloudABI__
#define CONFIG_HAS_ISATTY 1
#else
#define CONFIG_HAS_ISATTY 0
#endif

#ifndef __APPLE__
#define CONFIG_HAS_POSIX_FALLOCATE 1
#else
#define CONFIG_HAS_POSIX_FALLOCATE 0
#endif

#ifndef __APPLE__
#define CONFIG_HAS_PREADV 1
#else
#define CONFIG_HAS_PREADV 0
#endif

#if defined(__APPLE__) || defined(__CloudABI__)
#define CONFIG_HAS_PTHREAD_COND_TIMEDWAIT_RELATIVE_NP 1
#else
#define CONFIG_HAS_PTHREAD_COND_TIMEDWAIT_RELATIVE_NP 0
#endif

#ifndef __APPLE__
#define CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK 1
#else
#define CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK 0
#endif

#ifndef __APPLE__
#define CONFIG_HAS_PWRITEV 1
#else
#define CONFIG_HAS_PWRITEV 0
#endif

#ifdef __APPLE__
#define st_atimespec st_atim
#define st_mtimespec st_mtim
#define st_ctimespec st_ctim
#endif

#ifdef __APPLE__
#define CONFIG_TLS_USE_GSBASE 1
#else
#define CONFIG_TLS_USE_GSBASE 0
#endif

#endif
