#ifndef signal_handlers_h
#define signal_handlers_h

#include <stdint.h>
#include <setjmp.h>
#ifndef __cplusplus
#include <stdbool.h>
#endif

#include <signal.h>

#ifdef __cplusplus
extern "C" {
#endif

#if defined(_WIN32)
#include <windows.h>
#include <winternl.h>
void* HandleTrap(const uint8_t*, LPEXCEPTION_POINTERS);
#else
void* HandleTrap(const uint8_t*, int, siginfo_t *, void *);
#endif

void Unwind(void*);

// This function performs the low-overhead signal handler initialization that we
// want to do eagerly to ensure a more-deterministic global process state. This
// is especially relevant for signal handlers since handler ordering depends on
// installation order: the wasm signal handler must run *before* the other crash
// handlers and since POSIX signal handlers work LIFO, this function needs to be
// called at the end of the startup process, after other handlers have been
// installed. This function can thus be called multiple times, having no effect
// after the first call.
int
EnsureEagerSignalHandlers(void);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // signal_handlers_h
