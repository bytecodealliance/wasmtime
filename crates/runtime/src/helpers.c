#include <setjmp.h>
#include <stdint.h>
#include <stdlib.h>

// Note that `sigsetjmp` and `siglongjmp` are used here where possible to
// explicitly pass a 0 argument to `sigsetjmp` that we don't need to preserve
// the process signal mask. This should make this call a bit faster b/c it
// doesn't need to touch the kernel signal handling routines.
#ifdef CFG_TARGET_OS_windows
#define platform_setjmp(buf) setjmp(buf)
#define platform_longjmp(buf, arg) longjmp(buf, arg)
#define platform_jmp_buf jmp_buf
#else
#define platform_setjmp(buf) sigsetjmp(buf, 0)
#define platform_longjmp(buf, arg) siglongjmp(buf, arg)
#define platform_jmp_buf sigjmp_buf
#endif

int wasmtime_setjmp(
    void **buf_storage,
    void (*body)(void*, void*),
    void *payload,
    void *callee) {
  platform_jmp_buf buf;
  if (platform_setjmp(buf) != 0) {
    return 0;
  }
  *buf_storage = &buf;
  body(payload, callee);
  return 1;
}

void wasmtime_longjmp(void *JmpBuf) {
  platform_jmp_buf *buf = (platform_jmp_buf*) JmpBuf;
  platform_longjmp(*buf, 1);
}

// Just in case cross-language LTO is enabled we set the `noinline` attribute
// and also try to have some sort of side effect in this function with a dummy
// `asm` statement.
//
// Note the `weak` linkage here, though, which is intended to let other code
// override this symbol if it's defined elsewhere, since this definition doesn't
// matter.
#ifndef CFG_TARGET_OS_windows
__attribute__((weak, noinline))
#endif
void __jit_debug_register_code() {
#ifndef CFG_TARGET_OS_windows
  asm("");
#endif
}

struct JITDescriptor {
  uint32_t version_;
  uint32_t action_flag_;
  void* relevant_entry_;
  void* first_entry_;
};

// Note the `weak` linkage here which is the same purpose as above. We want to
// let other runtimes be able to override this since our own definition isn't
// important.
#ifndef CFG_TARGET_OS_windows
__attribute__((weak))
#endif
struct JITDescriptor __jit_debug_descriptor = {1, 0, NULL, NULL};

struct JITDescriptor* wasmtime_jit_debug_descriptor() {
  return &__jit_debug_descriptor;
}
