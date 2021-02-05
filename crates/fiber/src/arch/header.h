#ifndef __wasmtime_common_h
#define __wasmtime_common_h

#if CFG_TARGET_OS_macos

.section __TEXT,__text,regular,pure_instructions

#define GLOBL(fnname) .globl _##fnname
#define HIDDEN(fnname) .private_extern _##fnname
#define TYPE(fnname)
#define FUNCTION(fnname) _##fnname
#define SIZE(fnname)

// Tells the linker it's safe to gc symbols away if not used.
#define FOOTER .subsections_via_symbols

#else

.text

#define GLOBL(fnname) .globl fnname
#define HIDDEN(fnname) .hidden fnname
#ifdef CFG_TARGET_ARCH_arm
#define TYPE(fnname) .type fnname,%function
#else
#define TYPE(fnname) .type fnname,@function
#endif
#define FUNCTION(fnname) fnname
#define SIZE(fnname) .size fnname,.-fnname

// Mark that we don't need executable stack.
#define FOOTER .section .note.GNU-stack,"",%progbits

#endif

#endif // __wasmtime_common_h
