// Part of the Wasmtime Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://github.com/CraneStation/wasmtime/blob/master/LICENSE for license information.
//
// Significant parts of this file are derived from cloudabi-utils. See
// https://github.com/CraneStation/wasmtime/blob/master/lib/wasi/sandboxed-system-primitives/src/LICENSE
// for license information.
//
// The upstream file contains the following copyright notice:
//
// Copyright (c) 2016 Nuxi, https://nuxi.nl/

#ifndef REFCOUNT_H
#define REFCOUNT_H

#include <assert.h>
#if !defined(__GNUC__)
#include <stdatomic.h>
#endif
#include <stdbool.h>

#include "locking.h"

// Simple reference counter.
struct LOCKABLE refcount {
#if defined(__GNUC__)
  unsigned count;
#else
  atomic_uint count;
#endif
};

#define PRODUCES(...) LOCKS_SHARED(__VA_ARGS__) NO_LOCK_ANALYSIS
#define CONSUMES(...) UNLOCKS(__VA_ARGS__) NO_LOCK_ANALYSIS

// Initialize the reference counter.
static void refcount_init(struct refcount *r, unsigned int count) PRODUCES(*r) {
#if defined(__GNUC__)
  __atomic_store_n(&r->count, count, __ATOMIC_SEQ_CST);
#else
  atomic_init(&r->count, count);
#endif
}

// Increment the reference counter.
static inline void refcount_acquire(struct refcount *r) PRODUCES(*r) {
#if defined(__GNUC__)
  __atomic_fetch_add(&r->count, 1, __ATOMIC_ACQUIRE);
#else
  atomic_fetch_add_explicit(&r->count, 1, memory_order_acquire);
#endif
}

// Decrement the reference counter, returning whether the reference
// dropped to zero.
static inline bool refcount_release(struct refcount *r) CONSUMES(*r) {
#if defined(__GNUC__)
  int old = __atomic_fetch_sub(&r->count, 1, __ATOMIC_RELEASE);
#else
  int old = atomic_fetch_sub_explicit(&r->count, 1, memory_order_release);
#endif
  assert(old != 0 && "Reference count becoming negative");
  return old == 1;
}

#endif
