#include <assert.h>
#include <setjmp.h>
#include <signal.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/ucontext.h>
#include <unistd.h>

#include "wasmtime-platform.h"

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

uint8_t *wasmtime_mmap_new(uintptr_t size, uint32_t prot_flags) {
  void *rc = mmap(NULL, size, wasmtime_to_mmap_prot_flags(prot_flags),
                  MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  assert(rc != MAP_FAILED);
  return rc;
}

void wasmtime_mmap_remap(uint8_t *addr, uintptr_t size, uint32_t prot_flags) {
  void *rc = mmap(addr, size, wasmtime_to_mmap_prot_flags(prot_flags),
                  MAP_FIXED | MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
  assert(rc == addr);
  (void)rc;
}

void wasmtime_munmap(uint8_t *ptr, uintptr_t size) {
  int rc = munmap(ptr, size);
  assert(rc == 0);
  (void)rc;
}

void wasmtime_mprotect(uint8_t *ptr, uintptr_t size, uint32_t prot_flags) {
  int rc = mprotect(ptr, size, wasmtime_to_mmap_prot_flags(prot_flags));
  assert(rc == 0);
  (void)rc;
}

uintptr_t wasmtime_page_size(void) { return sysconf(_SC_PAGESIZE); }

int32_t wasmtime_setjmp(const uint8_t **jmp_buf_out,
                        void (*callback)(uint8_t *, uint8_t *),
                        uint8_t *payload, uint8_t *callee) {
  jmp_buf buf;
  if (setjmp(buf) != 0)
    return 0;
  *jmp_buf_out = (uint8_t *)&buf;
  callback(payload, callee);
  return 1;
}

void wasmtime_longjmp(const uint8_t *jmp_buf_ptr) {
  longjmp(*(jmp_buf *)jmp_buf_ptr, 1);
}

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

extern void wasmtime_init_traps(wasmtime_trap_handler_t handler) {
  int rc;
  g_handler = handler;

  struct sigaction action;
  memset(&action, 0, sizeof(action));

  action.sa_sigaction = handle_signal;
  action.sa_flags = SA_SIGINFO | SA_NODEFER;
  sigemptyset(&action.sa_mask);

  rc = sigaction(SIGILL, &action, NULL);
  assert(rc == 0);
  rc = sigaction(SIGSEGV, &action, NULL);
  assert(rc == 0);
  rc = sigaction(SIGFPE, &action, NULL);
  assert(rc == 0);
  (void)rc;
}

struct wasmtime_memory_image *wasmtime_memory_image_new(const uint8_t *ptr,
                                                        uintptr_t len) {
  return NULL;
}

void wasmtime_memory_image_map_at(struct wasmtime_memory_image *image,
                                  uint8_t *addr, uintptr_t len) {
  abort();
}

void wasmtime_memory_image_free(struct wasmtime_memory_image *image) {
  abort();
}
