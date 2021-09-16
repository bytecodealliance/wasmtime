#include <setjmp.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef CFG_TARGET_OS_windows
#define platform_setjmp(buf) setjmp(buf)
#define platform_longjmp(buf, arg) longjmp(buf, arg)
typedef jmp_buf platform_jmp_buf;
#else
// GCC and Clang both provide `__builtin_setjmp`/`__builtin_longjmp`, which
// differ from plain `setjmp` and `longjmp` in that they're implemented by
// the compiler inline rather than in libc, and the compiler can avoid saving
// and restoring most of the registers. See the [GCC docs] and [clang docs]
// for more information.
//
// [GCC docs]: https://gcc.gnu.org/onlinedocs/gcc/Nonlocal-Gotos.html
// [clang docs]: https://llvm.org/docs/ExceptionHandling.html#llvm-eh-sjlj-setjmp
#define platform_setjmp(buf) __builtin_setjmp(buf)
#define platform_longjmp(buf, arg) __builtin_longjmp(buf, arg)
typedef void *platform_jmp_buf[5]; // this is the documented size; see the docs links for details.
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
