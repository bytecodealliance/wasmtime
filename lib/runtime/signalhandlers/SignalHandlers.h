#ifndef signal_handlers_h
#define signal_handlers_h

#include <stdint.h>
#include <setjmp.h>
#ifndef __cplusplus
#include <stdbool.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

struct CodeSegment;

// Record the Trap code and wasm bytecode offset in TLS somewhere
void RecordTrap(const uint8_t* pc, const struct CodeSegment* codeSegment);

// Initiate an unwind.
void Unwind(void);

// Return the CodeSegment containing the given pc, if any exist in the process.
// This method does not take a lock.
const struct CodeSegment*
LookupCodeSegment(const void* pc);

// Trap initialization state.
struct TrapContext {
    bool triedToInstallSignalHandlers;
    bool haveSignalHandlers;
};

// This function performs the low-overhead signal handler initialization that we
// want to do eagerly to ensure a more-deterministic global process state. This
// is especially relevant for signal handlers since handler ordering depends on
// installation order: the wasm signal handler must run *before* the other crash
// handlers and since POSIX signal handlers work LIFO, this function needs to be
// called at the end of the startup process, after other handlers have been
// installed. This function can thus be called multiple times, having no effect
// after the first call.
bool
EnsureEagerSignalHandlers(void);

// Assuming EnsureEagerProcessSignalHandlers() has already been called,
// this function performs the full installation of signal handlers which must
// be performed per-thread. This operation may incur some overhead and
// so should be done only when needed to use wasm.
bool
EnsureDarwinMachPorts(void);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // signal_handlers_h
