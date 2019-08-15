// longjmp implementation on Windows unwinds the stack, but cranelift
// doesn't generate appropriate unwind info at the moment.
// Wasm stack frames don't require any cleanup for now, so we can
// simply restore stack and PC registers (__builtin_{set,long}jmp).

#include "SignalHandlers.hpp"

int WasmtimeSjljCallTrampoline(void *buf, void *vmctx, void(body)(void*, void*), void* args) {
	if (__builtin_setjmp(buf) != 0) {
		return 1;
	}
	PushJmpBuffer(buf);
	body(vmctx, args);
	return 0;
}

int WasmtimeSjljCall(void *buf, void *vmctx, void(body)(void*)) {
	if (__builtin_setjmp(buf) != 0) {
		return 1;
	}
	PushJmpBuffer(buf);
	body(vmctx);
	return 0;
}

_Noreturn void SjljUnwind(void *buf) {
	__builtin_longjmp(buf, 1);
}
