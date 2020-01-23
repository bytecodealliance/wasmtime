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

// Record the Trap code and wasm bytecode offset in TLS somewhere
void* RecordTrap(const uint8_t* pc, bool reset_guard_page);

#if defined(_WIN32)
#include <windows.h>
#include <winternl.h>
bool InstanceSignalHandler(LPEXCEPTION_POINTERS);
#elif defined(USE_APPLE_MACH_PORTS)
bool InstanceSignalHandler(int, siginfo_t *, void *);
#else
#include <sys/ucontext.h>
bool InstanceSignalHandler(int, siginfo_t *, ucontext_t *);
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
