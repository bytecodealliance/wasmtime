#include <unistd.h>
#include <setjmp.h>
#include <stdlib.h>
#include <sys/mman.h>

#include "SignalHandlers.hpp"

// The size of the sigaltstack (not including the guard, which will be added).
// Make this large enough to run our signal handlers.
static const size_t sigaltstack_size = 4 * SIGSTKSZ;

// A utility to register a new sigaltstack.
namespace {
  static thread_local class SigAltStack {
    size_t guard_size;
    size_t sigaltstack_alloc_size;
    stack_t new_stack;

  public:
    SigAltStack();
    ~SigAltStack();
  } thread_sigaltstack;
}

SigAltStack::SigAltStack()
  : guard_size(sysconf(_SC_PAGESIZE))
  , sigaltstack_alloc_size(guard_size + sigaltstack_size)
{
  // Allocate memory.
  void *ptr = mmap(NULL, sigaltstack_alloc_size, PROT_NONE,
                   MAP_PRIVATE | MAP_ANON, -1, 0);
  if (ptr == MAP_FAILED)
    RaiseOOMTrap();

  // Prepare the stack, register it, and sanity check the old stack.
  void *stack_ptr = (void *)((uintptr_t)ptr + guard_size);
  new_stack = (stack_t) { stack_ptr, 0, sigaltstack_size };
  stack_t old_stack;
  if (mprotect(stack_ptr, sigaltstack_size, PROT_READ | PROT_WRITE) != 0 ||
      sigaltstack(&new_stack, &old_stack) != 0 ||
      old_stack.ss_flags != 0 ||
      old_stack.ss_size > sigaltstack_size)
    abort();
}

SigAltStack::~SigAltStack() {
  // Disable the sigaltstack. We don't restore the old sigaltstack because
  // Rust may have restored its old sigaltstack already (the Rust at_exit
  // mechanism doesn't interleave with __cxa_atexit). Fortunately, the thread
  // is exiting so there's no need; we just make sure our sigaltstack is no
  // longer registered before we free it.
  static const stack_t disable_stack = { NULL, SS_DISABLE, SIGSTKSZ };
  void *alloc_ptr = (void *)((uintptr_t)new_stack.ss_sp - guard_size);
  if (sigaltstack(&disable_stack, NULL) != 0 ||
      munmap(alloc_ptr, sigaltstack_alloc_size) != 0)
    abort();
}

extern "C"
int RegisterSetjmp(
    void **buf_storage,
    void (*body)(void*),
    void *payload) {
  // Ensure that the thread-local sigaltstack is initialized.
  thread_sigaltstack;

  jmp_buf buf;
  if (setjmp(buf) != 0) {
    return 0;
  }
  *buf_storage = &buf;
  body(payload);
  return 1;
}

extern "C"
void Unwind(void *JmpBuf) {
  jmp_buf *buf = (jmp_buf*) JmpBuf;
  longjmp(*buf, 1);
}
