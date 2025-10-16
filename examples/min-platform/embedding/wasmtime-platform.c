#include <assert.h>
#include <errno.h>
#include <signal.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/ucontext.h>
#include <unistd.h>

#include "wasmtime-platform.h"

#ifdef WASMTIME_VIRTUAL_MEMORY

static int wasmtime_to_mmap_prot_flags(uint32_t prot_flags) {
  int flags = 0;
  if (prot_flags & WASMTIME_PROT_READ)
    flags |= PROT_READ;
  if (prot_flags & WASMTIME_PROT_WRITE)
    flags |= PROT_WRITE;
  if (prot_flags & WASMTIME_PROT_EXEC)
    flags |= PROT_EXEC;
  return flags;
}

int wasmtime_mmap_new(uintptr_t size, uint32_t prot_flags, uint8_t **ret) {
  void *rc = mmap(NULL, size, wasmtime_to_mmap_prot_flags(prot_flags),
                  MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (rc == MAP_FAILED)
    return errno;
  *ret = rc;
  return 0;
}

int wasmtime_mmap_remap(uint8_t *addr, uintptr_t size, uint32_t prot_flags) {
  void *rc = mmap(addr, size, wasmtime_to_mmap_prot_flags(prot_flags),
                  MAP_FIXED | MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  if (rc == MAP_FAILED)
    return errno;
  return 0;
}

int wasmtime_munmap(uint8_t *ptr, uintptr_t size) {
  int rc = munmap(ptr, size);
  if (rc != 0)
    return errno;
  return 0;
}

int wasmtime_mprotect(uint8_t *ptr, uintptr_t size, uint32_t prot_flags) {
  int rc = mprotect(ptr, size, wasmtime_to_mmap_prot_flags(prot_flags));
  if (rc != 0)
    return errno;
  return 0;
}

uintptr_t wasmtime_page_size(void) { return sysconf(_SC_PAGESIZE); }

#endif // WASMTIME_VIRTUAL_MEMORY

#ifdef WASMTIME_NATIVE_SIGNALS

static wasmtime_trap_handler_t g_handler = NULL;

static void handle_signal(int signo, siginfo_t *info, void *context) {
  assert(g_handler != NULL);
  uintptr_t ip, fp;
#if defined(__aarch64__)
  ucontext_t *cx = context;
  ip = cx->uc_mcontext.pc;
  fp = cx->uc_mcontext.regs[29];
#elif defined(__x86_64__)
  ucontext_t *cx = context;
  ip = cx->uc_mcontext.gregs[REG_RIP];
  fp = cx->uc_mcontext.gregs[REG_RBP];
#else
#error "Unsupported platform"
#endif

  bool has_faulting_addr = signo == SIGSEGV;
  uintptr_t faulting_addr = 0;
  if (has_faulting_addr)
    faulting_addr = (uintptr_t)info->si_addr;
  g_handler(ip, fp, has_faulting_addr, faulting_addr);

  // If wasmtime didn't handle this trap then reset the handler to the default
  // behavior which will probably abort the process.
  signal(signo, SIG_DFL);
}

int wasmtime_init_traps(wasmtime_trap_handler_t handler) {
  int rc;
  g_handler = handler;

  struct sigaction action;
  memset(&action, 0, sizeof(action));

  action.sa_sigaction = handle_signal;
  action.sa_flags = SA_SIGINFO | SA_NODEFER;
  sigemptyset(&action.sa_mask);

  rc = sigaction(SIGILL, &action, NULL);
  if (rc != 0)
    return errno;
  rc = sigaction(SIGSEGV, &action, NULL);
  if (rc != 0)
    return errno;
  rc = sigaction(SIGFPE, &action, NULL);
  if (rc != 0)
    return errno;
  return 0;
}

#endif // WASMTIME_NATIVE_SIGNALS

#ifdef WASMTIME_VIRTUAL_MEMORY

int wasmtime_memory_image_new(const uint8_t *ptr, uintptr_t len,
                              struct wasmtime_memory_image **ret) {
  *ret = NULL;
  return 0;
}

int wasmtime_memory_image_map_at(struct wasmtime_memory_image *image,
                                 uint8_t *addr, uintptr_t len) {
  abort();
}

void wasmtime_memory_image_free(struct wasmtime_memory_image *image) {
  abort();
}

#endif // WASMTIME_VIRTUAL_MEMORY

#ifdef WASMTIME_CUSTOM_SYNC

// Multi-threaded TLS using pthread
#include <pthread.h>

static pthread_key_t wasmtime_tls_key;
static pthread_once_t wasmtime_tls_key_once = PTHREAD_ONCE_INIT;

static void make_tls_key(void) { pthread_key_create(&wasmtime_tls_key, NULL); }

uint8_t *wasmtime_tls_get(void) {
  pthread_once(&wasmtime_tls_key_once, make_tls_key);
  return (uint8_t *)pthread_getspecific(wasmtime_tls_key);
}

void wasmtime_tls_set(uint8_t *val) {
  pthread_once(&wasmtime_tls_key_once, make_tls_key);
  pthread_setspecific(wasmtime_tls_key, val);
}

#else

// Single-threaded TLS using a static variable
static uint8_t *WASMTIME_TLS = NULL;

uint8_t *wasmtime_tls_get(void) { return WASMTIME_TLS; }

void wasmtime_tls_set(uint8_t *val) { WASMTIME_TLS = val; }

#endif

#ifdef WASMTIME_CUSTOM_SYNC

#include <stdlib.h>

void wasmtime_sync_lock_free(uintptr_t *lock) {
  if (*lock != 0) {
    pthread_mutex_t *mutex = (pthread_mutex_t *)*lock;
    pthread_mutex_destroy(mutex);
    free(mutex);
    *lock = 0;
  }
}

static pthread_mutex_t *mutex_lazy_init(uintptr_t *lock) {
  pthread_mutex_t *mutex = (pthread_mutex_t *)*lock;
  if (mutex == NULL) {
    pthread_mutex_t *new_mutex = malloc(sizeof(pthread_mutex_t));
    if (new_mutex == NULL) {
      abort();
    }
    pthread_mutex_init(new_mutex, NULL);

    // Atomically set lock only if it's still NULL
    if (!__sync_bool_compare_and_swap(lock, 0, (uintptr_t)new_mutex)) {
      pthread_mutex_destroy(new_mutex);
      free(new_mutex);
    }
    mutex = (pthread_mutex_t *)*lock;
  }
  return mutex;
}

void wasmtime_sync_lock_acquire(uintptr_t *lock) {
  pthread_mutex_t *mutex = mutex_lazy_init(lock);
  pthread_mutex_lock(mutex);
}

void wasmtime_sync_lock_release(uintptr_t *lock) {
  pthread_mutex_t *mutex = mutex_lazy_init(lock);
  pthread_mutex_unlock(mutex);
}

void wasmtime_sync_rwlock_free(uintptr_t *lock) {
  if (*lock != 0) {
    pthread_rwlock_t *rwlock = (pthread_rwlock_t *)*lock;
    pthread_rwlock_destroy(rwlock);
    free(rwlock);
    *lock = 0;
  }
}

static pthread_rwlock_t *rwlock_lazy_init(uintptr_t *lock) {
  pthread_rwlock_t *rwlock = (pthread_rwlock_t *)*lock;
  if (rwlock == NULL) {
    pthread_rwlock_t *new_rwlock = malloc(sizeof(pthread_rwlock_t));
    if (new_rwlock == NULL) {
      abort();
    }
    pthread_rwlock_init(new_rwlock, NULL);

    // Atomically set lock only if it's still NULL
    if (!__sync_bool_compare_and_swap(lock, 0, (uintptr_t)new_rwlock)) {
      // Another thread won the race, discard our allocation
      pthread_rwlock_destroy(new_rwlock);
      free(new_rwlock);
    }
    rwlock = (pthread_rwlock_t *)*lock;
  }
  return rwlock;
}

void wasmtime_sync_rwlock_read(uintptr_t *lock) {
  pthread_rwlock_t *rwlock = rwlock_lazy_init(lock);
  pthread_rwlock_rdlock(rwlock);
}

void wasmtime_sync_rwlock_read_release(uintptr_t *lock) {
  pthread_rwlock_t *rwlock = rwlock_lazy_init(lock);
  pthread_rwlock_unlock(rwlock);
}

void wasmtime_sync_rwlock_write(uintptr_t *lock) {
  pthread_rwlock_t *rwlock = rwlock_lazy_init(lock);
  pthread_rwlock_wrlock(rwlock);
}

void wasmtime_sync_rwlock_write_release(uintptr_t *lock) {
  pthread_rwlock_t *rwlock = rwlock_lazy_init(lock);
  pthread_rwlock_unlock(rwlock);
}

#endif // WASMTIME_CUSTOM_SYNC
