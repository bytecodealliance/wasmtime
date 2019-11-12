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

#ifndef LOCKING_H
#define LOCKING_H

#include "config.h"

#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <stdint.h>
#include <time.h>

#ifndef __has_extension
#define __has_extension(x) 0
#endif

#if __has_extension(c_thread_safety_attributes)
#define LOCK_ANNOTATE(x) __attribute__((x))
#else
#define LOCK_ANNOTATE(x)
#endif

// Lock annotation macros.

#define LOCKABLE LOCK_ANNOTATE(lockable)

#define LOCKS_EXCLUSIVE(...) LOCK_ANNOTATE(exclusive_lock_function(__VA_ARGS__))
#define LOCKS_SHARED(...) LOCK_ANNOTATE(shared_lock_function(__VA_ARGS__))

#define TRYLOCKS_EXCLUSIVE(...) \
  LOCK_ANNOTATE(exclusive_trylock_function(__VA_ARGS__))
#define TRYLOCKS_SHARED(...) LOCK_ANNOTATE(shared_trylock_function(__VA_ARGS__))

#define UNLOCKS(...) LOCK_ANNOTATE(unlock_function(__VA_ARGS__))

#define REQUIRES_EXCLUSIVE(...) \
  LOCK_ANNOTATE(exclusive_locks_required(__VA_ARGS__))
#define REQUIRES_SHARED(...) LOCK_ANNOTATE(shared_locks_required(__VA_ARGS__))
#define REQUIRES_UNLOCKED(...) LOCK_ANNOTATE(locks_excluded(__VA_ARGS__))

#define NO_LOCK_ANALYSIS LOCK_ANNOTATE(no_thread_safety_analysis)

// Mutex that uses the lock annotations.

struct LOCKABLE mutex {
  pthread_mutex_t object;
};

#define MUTEX_INITIALIZER \
  { PTHREAD_MUTEX_INITIALIZER }

static inline void mutex_init(struct mutex *lock) REQUIRES_UNLOCKED(*lock) {
  pthread_mutex_init(&lock->object, NULL);
}

static inline void mutex_destroy(struct mutex *lock) REQUIRES_UNLOCKED(*lock) {
  pthread_mutex_destroy(&lock->object);
}

static inline void mutex_lock(struct mutex *lock)
    LOCKS_EXCLUSIVE(*lock) NO_LOCK_ANALYSIS {
  pthread_mutex_lock(&lock->object);
}

static inline void mutex_unlock(struct mutex *lock)
    UNLOCKS(*lock) NO_LOCK_ANALYSIS {
  pthread_mutex_unlock(&lock->object);
}

// Read-write lock that uses the lock annotations.

struct LOCKABLE rwlock {
  pthread_rwlock_t object;
};

static inline void rwlock_init(struct rwlock *lock) REQUIRES_UNLOCKED(*lock) {
  pthread_rwlock_init(&lock->object, NULL);
}

static inline void rwlock_rdlock(struct rwlock *lock)
    LOCKS_SHARED(*lock) NO_LOCK_ANALYSIS {
  pthread_rwlock_rdlock(&lock->object);
}

static inline void rwlock_wrlock(struct rwlock *lock)
    LOCKS_EXCLUSIVE(*lock) NO_LOCK_ANALYSIS {
  pthread_rwlock_wrlock(&lock->object);
}

static inline void rwlock_unlock(struct rwlock *lock)
    UNLOCKS(*lock) NO_LOCK_ANALYSIS {
  pthread_rwlock_unlock(&lock->object);
}

// Condition variable that uses the lock annotations.

struct LOCKABLE cond {
  pthread_cond_t object;
#if !CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK || \
    !CONFIG_HAS_PTHREAD_COND_TIMEDWAIT_RELATIVE_NP
  clockid_t clock;
#endif
};

static inline void cond_init_monotonic(struct cond *cond) {
#if CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK
  pthread_condattr_t attr;
  pthread_condattr_init(&attr);
  pthread_condattr_setclock(&attr, CLOCK_MONOTONIC);
  pthread_cond_init(&cond->object, &attr);
  pthread_condattr_destroy(&attr);
#else
  pthread_cond_init(&cond->object, NULL);
#endif
#if !CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK || \
    !CONFIG_HAS_PTHREAD_COND_TIMEDWAIT_RELATIVE_NP
  cond->clock = CLOCK_MONOTONIC;
#endif
}

static inline void cond_init_realtime(struct cond *cond) {
  pthread_cond_init(&cond->object, NULL);
#if !CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK || \
    !CONFIG_HAS_PTHREAD_COND_TIMEDWAIT_RELATIVE_NP
  cond->clock = CLOCK_REALTIME;
#endif
}

static inline void cond_destroy(struct cond *cond) {
  pthread_cond_destroy(&cond->object);
}

static inline void cond_signal(struct cond *cond) {
  pthread_cond_signal(&cond->object);
}

static inline bool cond_timedwait(struct cond *cond, struct mutex *lock,
                                  uint64_t timeout, bool abstime)
    REQUIRES_EXCLUSIVE(*lock) NO_LOCK_ANALYSIS {
  struct timespec ts = {
      .tv_sec = (time_t)(timeout / 1000000000),
      .tv_nsec = (long)(timeout % 1000000000),
  };

  if (abstime) {
#if !CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK
    // No native support for sleeping on monotonic clocks. Convert the
    // timeout to a relative value and then to an absolute value for the
    // realtime clock.
    if (cond->clock != CLOCK_REALTIME) {
      struct timespec ts_monotonic;
      clock_gettime(cond->clock, &ts_monotonic);
      ts.tv_sec -= ts_monotonic.tv_sec;
      ts.tv_nsec -= ts_monotonic.tv_nsec;
      if (ts.tv_nsec < 0) {
        ts.tv_nsec += 1000000000;
        --ts.tv_sec;
      }

      struct timespec ts_realtime;
      clock_gettime(CLOCK_REALTIME, &ts_realtime);
      ts.tv_sec += ts_realtime.tv_sec;
      ts.tv_nsec += ts_realtime.tv_nsec;
      if (ts.tv_nsec >= 1000000000) {
        ts.tv_nsec -= 1000000000;
        ++ts.tv_sec;
      }
    }
#endif
  } else {
#if CONFIG_HAS_PTHREAD_COND_TIMEDWAIT_RELATIVE_NP
    // Implementation supports relative timeouts.
    int ret =
        pthread_cond_timedwait_relative_np(&cond->object, &lock->object, &ts);
    assert((ret == 0 || ret == ETIMEDOUT) &&
           "pthread_cond_timedwait_relative_np() failed");
    return ret == ETIMEDOUT;
#else
    // Convert to absolute timeout.
    struct timespec ts_now;
#if CONFIG_HAS_PTHREAD_CONDATTR_SETCLOCK
    clock_gettime(cond->clock, &ts_now);
#else
    clock_gettime(CLOCK_REALTIME, &ts_now);
#endif
    ts.tv_sec += ts_now.tv_sec;
    ts.tv_nsec += ts_now.tv_nsec;
    if (ts.tv_nsec >= 1000000000) {
      ts.tv_nsec -= 1000000000;
      ++ts.tv_sec;
    }
#endif
  }

  int ret = pthread_cond_timedwait(&cond->object, &lock->object, &ts);
  assert((ret == 0 || ret == ETIMEDOUT) && "pthread_cond_timedwait() failed");
  return ret == ETIMEDOUT;
}

static inline void cond_wait(struct cond *cond, struct mutex *lock)
    REQUIRES_EXCLUSIVE(*lock) NO_LOCK_ANALYSIS {
  pthread_cond_wait(&cond->object, &lock->object);
}

#endif
