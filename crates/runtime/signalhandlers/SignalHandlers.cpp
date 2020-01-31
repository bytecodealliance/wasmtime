//! This file is largely derived from the code in WasmSignalHandlers.cpp in SpiderMonkey:
//!
//! https://dxr.mozilla.org/mozilla-central/source/js/src/wasm/WasmSignalHandlers.cpp
//!
//! Use of Mach ports on Darwin platforms (the USE_APPLE_MACH_PORTS code below) is
//! currently disabled.

#include "SignalHandlers.hpp"

#include <stdint.h>
#include <assert.h>
#include <stdlib.h>
#include <stdio.h>

#if defined(_WIN32)

# include <windows.h>
# include <winternl.h>

#elif defined(USE_APPLE_MACH_PORTS)
# include <mach/exc.h>
# include <mach/mach.h>
# include <pthread.h>
#else
# include <signal.h>
#endif

// =============================================================================
// This following pile of macros and includes defines the ToRegisterState() and
// the ContextToPC() functions from the (highly) platform-specific CONTEXT
// struct which is provided to the signal handler.
// =============================================================================

#if defined(__FreeBSD__) || defined(__FreeBSD_kernel__)
# include <sys/ucontext.h> // for ucontext_t, mcontext_t
#endif

#if defined(__x86_64__)
# if defined(__DragonFly__)
#  include <machine/npx.h> // for union savefpu
# elif defined(__FreeBSD__) || defined(__FreeBSD_kernel__) || \
       defined(__NetBSD__) || defined(__OpenBSD__)
#  include <machine/fpu.h> // for struct savefpu/fxsave64
# endif
#endif

#if defined(_WIN32)
# define EIP_sig(p) ((p)->Eip)
# define EBP_sig(p) ((p)->Ebp)
# define ESP_sig(p) ((p)->Esp)
# define RIP_sig(p) ((p)->Rip)
# define RSP_sig(p) ((p)->Rsp)
# define RBP_sig(p) ((p)->Rbp)
# define R11_sig(p) ((p)->R11)
# define R13_sig(p) ((p)->R13)
# define R14_sig(p) ((p)->R14)
# define R15_sig(p) ((p)->R15)
# define EPC_sig(p) ((p)->Pc)
# define RFP_sig(p) ((p)->Fp)
# define R31_sig(p) ((p)->Sp)
# define RLR_sig(p) ((p)->Lr)
#elif defined(__OpenBSD__)
# define EIP_sig(p) ((p)->sc_eip)
# define EBP_sig(p) ((p)->sc_ebp)
# define ESP_sig(p) ((p)->sc_esp)
# define RIP_sig(p) ((p)->sc_rip)
# define RSP_sig(p) ((p)->sc_rsp)
# define RBP_sig(p) ((p)->sc_rbp)
# define R11_sig(p) ((p)->sc_r11)
# if defined(__arm__)
#  define R13_sig(p) ((p)->sc_usr_sp)
#  define R14_sig(p) ((p)->sc_usr_lr)
#  define R15_sig(p) ((p)->sc_pc)
# else
#  define R13_sig(p) ((p)->sc_r13)
#  define R14_sig(p) ((p)->sc_r14)
#  define R15_sig(p) ((p)->sc_r15)
# endif
# if defined(__aarch64__)
#  define EPC_sig(p) ((p)->sc_elr)
#  define RFP_sig(p) ((p)->sc_x[29])
#  define RLR_sig(p) ((p)->sc_lr)
#  define R31_sig(p) ((p)->sc_sp)
# endif
# if defined(__mips__)
#  define EPC_sig(p) ((p)->sc_pc)
#  define RFP_sig(p) ((p)->sc_regs[30])
# endif
#elif defined(__linux__) || defined(__sun)
# if defined(__linux__)
#  define EIP_sig(p) ((p)->uc_mcontext.gregs[REG_EIP])
#  define EBP_sig(p) ((p)->uc_mcontext.gregs[REG_EBP])
#  define ESP_sig(p) ((p)->uc_mcontext.gregs[REG_ESP])
# else
#  define EIP_sig(p) ((p)->uc_mcontext.gregs[REG_PC])
#  define EBP_sig(p) ((p)->uc_mcontext.gregs[REG_EBP])
#  define ESP_sig(p) ((p)->uc_mcontext.gregs[REG_ESP])
# endif
# define RIP_sig(p) ((p)->uc_mcontext.gregs[REG_RIP])
# define RSP_sig(p) ((p)->uc_mcontext.gregs[REG_RSP])
# define RBP_sig(p) ((p)->uc_mcontext.gregs[REG_RBP])
# if defined(__linux__) && defined(__arm__)
#  define R11_sig(p) ((p)->uc_mcontext.arm_fp)
#  define R13_sig(p) ((p)->uc_mcontext.arm_sp)
#  define R14_sig(p) ((p)->uc_mcontext.arm_lr)
#  define R15_sig(p) ((p)->uc_mcontext.arm_pc)
# else
#  define R11_sig(p) ((p)->uc_mcontext.gregs[REG_R11])
#  define R13_sig(p) ((p)->uc_mcontext.gregs[REG_R13])
#  define R14_sig(p) ((p)->uc_mcontext.gregs[REG_R14])
#  define R15_sig(p) ((p)->uc_mcontext.gregs[REG_R15])
# endif
# if defined(__linux__) && defined(__aarch64__)
#  define EPC_sig(p) ((p)->uc_mcontext.pc)
#  define RFP_sig(p) ((p)->uc_mcontext.regs[29])
#  define RLR_sig(p) ((p)->uc_mcontext.regs[30])
#  define R31_sig(p) ((p)->uc_mcontext.regs[31])
# endif
# if defined(__linux__) && defined(__mips__)
#  define EPC_sig(p) ((p)->uc_mcontext.pc)
#  define RFP_sig(p) ((p)->uc_mcontext.gregs[30])
#  define RSP_sig(p) ((p)->uc_mcontext.gregs[29])
#  define R31_sig(p) ((p)->uc_mcontext.gregs[31])
# endif
# if defined(__linux__) && (defined(__sparc__) && defined(__arch64__))
#  define PC_sig(p) ((p)->uc_mcontext.mc_gregs[MC_PC])
#  define FP_sig(p) ((p)->uc_mcontext.mc_fp)
#  define SP_sig(p) ((p)->uc_mcontext.mc_i7)
# endif
# if defined(__linux__) && \
     (defined(__ppc64__) ||  defined (__PPC64__) || defined(__ppc64le__) || defined (__PPC64LE__))
#  define R01_sig(p) ((p)->uc_mcontext.gp_regs[1])
#  define R32_sig(p) ((p)->uc_mcontext.gp_regs[32])
# endif
#elif defined(__NetBSD__)
# define EIP_sig(p) ((p)->uc_mcontext.__gregs[_REG_EIP])
# define EBP_sig(p) ((p)->uc_mcontext.__gregs[_REG_EBP])
# define ESP_sig(p) ((p)->uc_mcontext.__gregs[_REG_ESP])
# define RIP_sig(p) ((p)->uc_mcontext.__gregs[_REG_RIP])
# define RSP_sig(p) ((p)->uc_mcontext.__gregs[_REG_RSP])
# define RBP_sig(p) ((p)->uc_mcontext.__gregs[_REG_RBP])
# define R11_sig(p) ((p)->uc_mcontext.__gregs[_REG_R11])
# define R13_sig(p) ((p)->uc_mcontext.__gregs[_REG_R13])
# define R14_sig(p) ((p)->uc_mcontext.__gregs[_REG_R14])
# define R15_sig(p) ((p)->uc_mcontext.__gregs[_REG_R15])
# if defined(__aarch64__)
#  define EPC_sig(p) ((p)->uc_mcontext.__gregs[_REG_PC])
#  define RFP_sig(p) ((p)->uc_mcontext.__gregs[_REG_X29])
#  define RLR_sig(p) ((p)->uc_mcontext.__gregs[_REG_X30])
#  define R31_sig(p) ((p)->uc_mcontext.__gregs[_REG_SP])
# endif
# if defined(__mips__)
#  define EPC_sig(p) ((p)->uc_mcontext.__gregs[_REG_EPC])
#  define RFP_sig(p) ((p)->uc_mcontext.__gregs[_REG_S8])
# endif
#elif defined(__DragonFly__) || defined(__FreeBSD__) || defined(__FreeBSD_kernel__)
# define EIP_sig(p) ((p)->uc_mcontext.mc_eip)
# define EBP_sig(p) ((p)->uc_mcontext.mc_ebp)
# define ESP_sig(p) ((p)->uc_mcontext.mc_esp)
# define RIP_sig(p) ((p)->uc_mcontext.mc_rip)
# define RSP_sig(p) ((p)->uc_mcontext.mc_rsp)
# define RBP_sig(p) ((p)->uc_mcontext.mc_rbp)
# if defined(__FreeBSD__) && defined(__arm__)
#  define R11_sig(p) ((p)->uc_mcontext.__gregs[_REG_R11])
#  define R13_sig(p) ((p)->uc_mcontext.__gregs[_REG_R13])
#  define R14_sig(p) ((p)->uc_mcontext.__gregs[_REG_R14])
#  define R15_sig(p) ((p)->uc_mcontext.__gregs[_REG_R15])
# else
#  define R11_sig(p) ((p)->uc_mcontext.mc_r11)
#  define R13_sig(p) ((p)->uc_mcontext.mc_r13)
#  define R14_sig(p) ((p)->uc_mcontext.mc_r14)
#  define R15_sig(p) ((p)->uc_mcontext.mc_r15)
# endif
# if defined(__FreeBSD__) && defined(__aarch64__)
#  define EPC_sig(p) ((p)->uc_mcontext.mc_gpregs.gp_elr)
#  define RFP_sig(p) ((p)->uc_mcontext.mc_gpregs.gp_x[29])
#  define RLR_sig(p) ((p)->uc_mcontext.mc_gpregs.gp_lr)
#  define R31_sig(p) ((p)->uc_mcontext.mc_gpregs.gp_sp)
# endif
# if defined(__FreeBSD__) && defined(__mips__)
#  define EPC_sig(p) ((p)->uc_mcontext.mc_pc)
#  define RFP_sig(p) ((p)->uc_mcontext.mc_regs[30])
# endif
#elif defined(USE_APPLE_MACH_PORTS)
# define EIP_sig(p) ((p)->thread.uts.ts32.__eip)
# define EBP_sig(p) ((p)->thread.uts.ts32.__ebp)
# define ESP_sig(p) ((p)->thread.uts.ts32.__esp)
# define RIP_sig(p) ((p)->thread.__rip)
# define RBP_sig(p) ((p)->thread.__rbp)
# define RSP_sig(p) ((p)->thread.__rsp)
# define R11_sig(p) ((p)->thread.__r[11])
# define R13_sig(p) ((p)->thread.__sp)
# define R14_sig(p) ((p)->thread.__lr)
# define R15_sig(p) ((p)->thread.__pc)
#elif defined(__APPLE__)
# define EIP_sig(p) ((p)->uc_mcontext->__ss.__eip)
# define EBP_sig(p) ((p)->uc_mcontext->__ss.__ebp)
# define ESP_sig(p) ((p)->uc_mcontext->__ss.__esp)
# define RIP_sig(p) ((p)->uc_mcontext->__ss.__rip)
# define RBP_sig(p) ((p)->uc_mcontext->__ss.__rbp)
# define RSP_sig(p) ((p)->uc_mcontext->__ss.__rsp)
# define R11_sig(p) ((p)->uc_mcontext->__ss.__r11)
# define R13_sig(p) ((p)->uc_mcontext->__ss.__sp)
# define R14_sig(p) ((p)->uc_mcontext->__ss.__lr)
# define R15_sig(p) ((p)->uc_mcontext->__ss.__pc)
#else
# error "Don't know how to read/write to the thread state via the mcontext_t."
#endif

#if defined(ANDROID)
// Not all versions of the Android NDK define ucontext_t or mcontext_t.
// Detect this and provide custom but compatible definitions. Note that these
// follow the GLibc naming convention to access register values from
// mcontext_t.
//
// See: https://chromiumcodereview.appspot.com/10829122/
// See: http://code.google.com/p/android/issues/detail?id=34784
# if !defined(__BIONIC_HAVE_UCONTEXT_T)
#  if defined(__arm__)

// GLibc on ARM defines mcontext_t has a typedef for 'struct sigcontext'.
// Old versions of the C library <signal.h> didn't define the type.
#   if !defined(__BIONIC_HAVE_STRUCT_SIGCONTEXT)
#    include <asm/sigcontext.h>
#   endif

typedef struct sigcontext mcontext_t;

typedef struct ucontext {
    uint32_t uc_flags;
    struct ucontext* uc_link;
    stack_t uc_stack;
    mcontext_t uc_mcontext;
    // Other fields are not used so don't define them here.
} ucontext_t;

#  elif defined(__mips__)

typedef struct {
    uint32_t regmask;
    uint32_t status;
    uint64_t pc;
    uint64_t gregs[32];
    uint64_t fpregs[32];
    uint32_t acx;
    uint32_t fpc_csr;
    uint32_t fpc_eir;
    uint32_t used_math;
    uint32_t dsp;
    uint64_t mdhi;
    uint64_t mdlo;
    uint32_t hi1;
    uint32_t lo1;
    uint32_t hi2;
    uint32_t lo2;
    uint32_t hi3;
    uint32_t lo3;
} mcontext_t;

typedef struct ucontext {
    uint32_t uc_flags;
    struct ucontext* uc_link;
    stack_t uc_stack;
    mcontext_t uc_mcontext;
    // Other fields are not used so don't define them here.
} ucontext_t;

#  elif defined(__i386__)
// x86 version for Android.
typedef struct {
    uint32_t gregs[19];
    void* fpregs;
    uint32_t oldmask;
    uint32_t cr2;
} mcontext_t;

typedef uint32_t kernel_sigset_t[2];  // x86 kernel uses 64-bit signal masks
typedef struct ucontext {
    uint32_t uc_flags;
    struct ucontext* uc_link;
    stack_t uc_stack;
    mcontext_t uc_mcontext;
    // Other fields are not used by V8, don't define them here.
} ucontext_t;
enum { REG_EIP = 14 };
#  endif  // defined(__i386__)
# endif  // !defined(__BIONIC_HAVE_UCONTEXT_T)
#endif // defined(ANDROID)

#if defined(USE_APPLE_MACH_PORTS)
# if defined(__x86_64__)
struct macos_x64_context {
    x86_thread_state64_t thread;
    x86_float_state64_t float_;
};
#  define CONTEXT macos_x64_context
# elif defined(__i386__)
struct macos_x86_context {
    x86_thread_state_t thread;
    x86_float_state_t float_;
};
#  define CONTEXT macos_x86_context
# elif defined(__arm__)
struct macos_arm_context {
    arm_thread_state_t thread;
    arm_neon_state_t float_;
};
#  define CONTEXT macos_arm_context
# else
#  error Unsupported architecture
# endif
#elif !defined(_WIN32)
# define CONTEXT ucontext_t
#endif

#if defined(_M_X64) || defined(__x86_64__)
# define PC_sig(p) RIP_sig(p)
# define FP_sig(p) RBP_sig(p)
# define SP_sig(p) RSP_sig(p)
#elif defined(_M_IX86) || defined(__i386__)
# define PC_sig(p) EIP_sig(p)
# define FP_sig(p) EBP_sig(p)
# define SP_sig(p) ESP_sig(p)
#elif defined(__arm__)
# define FP_sig(p) R11_sig(p)
# define SP_sig(p) R13_sig(p)
# define LR_sig(p) R14_sig(p)
# define PC_sig(p) R15_sig(p)
#elif defined(_M_ARM64) || defined(__aarch64__)
# define PC_sig(p) EPC_sig(p)
# define FP_sig(p) RFP_sig(p)
# define SP_sig(p) R31_sig(p)
# define LR_sig(p) RLR_sig(p)
#elif defined(__mips__)
# define PC_sig(p) EPC_sig(p)
# define FP_sig(p) RFP_sig(p)
# define SP_sig(p) RSP_sig(p)
# define LR_sig(p) R31_sig(p)
#elif defined(__ppc64__) ||  defined (__PPC64__) || defined(__ppc64le__) || defined (__PPC64LE__)
# define PC_sig(p) R32_sig(p)
# define SP_sig(p) R01_sig(p)
# define FP_sig(p) R01_sig(p)
#endif

static void
SetContextPC(CONTEXT* context, const uint8_t* pc)
{
#ifdef PC_sig
    PC_sig(context) = reinterpret_cast<uintptr_t>(pc);
#else
    abort();
#endif
}

static const uint8_t*
ContextToPC(CONTEXT* context)
{
#ifdef PC_sig
    return reinterpret_cast<const uint8_t*>(static_cast<uintptr_t>(PC_sig(context)));
#else
    abort();
#endif
}

// =============================================================================
// All signals/exceptions funnel down to this one trap-handling function which
// tests whether the pc is in a wasm module and, if so, whether there is
// actually a trap expected at this pc. These tests both avoid real bugs being
// silently converted to wasm traps and provides the trapping wasm bytecode
// offset we need to report in the error.
//
// Crashing inside wasm trap handling (due to a bug in trap handling or exposed
// during trap handling) must be reported like a normal crash, not cause the
// crash report to be lost. On Windows and non-Mach Unix, a crash during the
// handler reenters the handler, possibly repeatedly until exhausting the stack,
// and so we prevent recursion with the thread-local sAlreadyHandlingTrap. On
// Mach, the wasm exception handler has its own thread and is installed only on
// the thread-level debugging ports of our threads, so a crash on
// exception handler thread will not recurse; it will bubble up to the
// process-level debugging ports (where Breakpad is installed).
// =============================================================================

static thread_local bool sAlreadyHandlingTrap;

namespace {

struct AutoHandlingTrap
{
    AutoHandlingTrap() {
        assert(!sAlreadyHandlingTrap);
        sAlreadyHandlingTrap = true;
    }

    ~AutoHandlingTrap() {
        assert(sAlreadyHandlingTrap);
        sAlreadyHandlingTrap = false;
    }
};

}

// =============================================================================
// The following platform-specific handlers funnel all signals/exceptions into
// the HandleTrap() function defined in Rust. Note that the Rust function has a
// different ABI depending on the platform.
// =============================================================================

#if defined(_WIN32)
// Obtained empirically from thread_local codegen on x86/x64/arm64.
// Compiled in all user binaries, so should be stable over time.
static const unsigned sThreadLocalArrayPointerIndex = 11;

static LONG WINAPI
WasmTrapHandler(LPEXCEPTION_POINTERS exception)
{
    // Make sure TLS is initialized before reading sAlreadyHandlingTrap.
    if (!NtCurrentTeb()->Reserved1[sThreadLocalArrayPointerIndex]) {
        return EXCEPTION_CONTINUE_SEARCH;
    }

    if (sAlreadyHandlingTrap) {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    AutoHandlingTrap aht;

    EXCEPTION_RECORD* record = exception->ExceptionRecord;
    if (record->ExceptionCode != EXCEPTION_ACCESS_VIOLATION &&
        record->ExceptionCode != EXCEPTION_ILLEGAL_INSTRUCTION &&
        record->ExceptionCode != EXCEPTION_STACK_OVERFLOW &&
        record->ExceptionCode != EXCEPTION_INT_DIVIDE_BY_ZERO &&
        record->ExceptionCode != EXCEPTION_INT_OVERFLOW)
    {
        return EXCEPTION_CONTINUE_SEARCH;
    }

    void *JmpBuf = HandleTrap(ContextToPC(exception->ContextRecord), exception);
    // Test if a custom instance signal handler handled the exception
    if (((size_t) JmpBuf) == 1)
        return EXCEPTION_CONTINUE_EXECUTION;

    // Otherwise test if we need to longjmp to this buffer
    if (JmpBuf != nullptr) {
        sAlreadyHandlingTrap = false;
        Unwind(JmpBuf);
    }

    // ... and otherwise keep looking for a handler
    return EXCEPTION_CONTINUE_SEARCH;
}

#elif defined(USE_APPLE_MACH_PORTS)
// On OSX we are forced to use the lower-level Mach exception mechanism instead
// of Unix signals because breakpad uses Mach exceptions and would otherwise
// report a crash before wasm gets a chance to handle the exception.

// This definition was generated by mig (the Mach Interface Generator) for the
// routine 'exception_raise' (exc.defs).
#pragma pack(4)
typedef struct {
    mach_msg_header_t Head;
    /* start of the kernel processed data */
    mach_msg_body_t msgh_body;
    mach_msg_port_descriptor_t thread;
    mach_msg_port_descriptor_t task;
    /* end of the kernel processed data */
    NDR_record_t NDR;
    exception_type_t exception;
    mach_msg_type_number_t codeCnt;
    int64_t code[2];
} Request__mach_exception_raise_t;
#pragma pack()

// The full Mach message also includes a trailer.
struct ExceptionRequest
{
    Request__mach_exception_raise_t body;
    mach_msg_trailer_t trailer;
};

static bool
HandleMachException(const ExceptionRequest& request)
{
    // Get the port of the thread from the message.
    mach_port_t cxThread = request.body.thread.name;

    // Read out the thread's register state.
    CONTEXT context;
# if defined(__x86_64__)
    unsigned int thread_state_count = x86_THREAD_STATE64_COUNT;
    unsigned int float_state_count = x86_FLOAT_STATE64_COUNT;
    int thread_state = x86_THREAD_STATE64;
    int float_state = x86_FLOAT_STATE64;
# elif defined(__i386__)
    unsigned int thread_state_count = x86_THREAD_STATE_COUNT;
    unsigned int float_state_count = x86_FLOAT_STATE_COUNT;
    int thread_state = x86_THREAD_STATE;
    int float_state = x86_FLOAT_STATE;
# elif defined(__arm__)
    unsigned int thread_state_count = ARM_THREAD_STATE_COUNT;
    unsigned int float_state_count = ARM_NEON_STATE_COUNT;
    int thread_state = ARM_THREAD_STATE;
    int float_state = ARM_NEON_STATE;
# else
#  error Unsupported architecture
# endif
    kern_return_t kret;
    kret = thread_get_state(cxThread, thread_state,
                            (thread_state_t)&context.thread, &thread_state_count);
    if (kret != KERN_SUCCESS) {
        return false;
    }
    kret = thread_get_state(cxThread, float_state,
                            (thread_state_t)&context.float_, &float_state_count);
    if (kret != KERN_SUCCESS) {
        return false;
    }

    if (request.body.exception != EXC_BAD_ACCESS &&
        request.body.exception != EXC_BAD_INSTRUCTION)
    {
        return false;
    }

    {
        AutoHandlingTrap aht;
        if (!HandleTrap(&context, false)) {
            return false;
        }
    }

    // Update the thread state with the new pc and register values.
    kret = thread_set_state(cxThread, float_state, (thread_state_t)&context.float_, float_state_count);
    if (kret != KERN_SUCCESS) {
        return false;
    }
    kret = thread_set_state(cxThread, thread_state, (thread_state_t)&context.thread, thread_state_count);
    if (kret != KERN_SUCCESS) {
        return false;
    }

    return true;
}

static mach_port_t sMachDebugPort = MACH_PORT_NULL;

static void*
MachExceptionHandlerThread(void* arg)
{
    // Taken from mach_exc in /usr/include/mach/mach_exc.defs.
    static const unsigned EXCEPTION_MSG_ID = 2405;

    while (true) {
        ExceptionRequest request;
        kern_return_t kret = mach_msg(&request.body.Head, MACH_RCV_MSG, 0, sizeof(request),
                                      sMachDebugPort, MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);

        // If we fail even receiving the message, we can't even send a reply!
        // Rather than hanging the faulting thread (hanging the browser), crash.
        if (kret != KERN_SUCCESS) {
            fprintf(stderr, "MachExceptionHandlerThread: mach_msg failed with %d\n", (int)kret);
            abort();
        }

        if (request.body.Head.msgh_id != EXCEPTION_MSG_ID) {
            fprintf(stderr, "Unexpected msg header id %d\n", (int)request.body.Head.msgh_bits);
            abort();
        }

        // Some thread just commited an EXC_BAD_ACCESS and has been suspended by
        // the kernel. The kernel is waiting for us to reply with instructions.
        // Our default is the "not handled" reply (by setting the RetCode field
        // of the reply to KERN_FAILURE) which tells the kernel to continue
        // searching at the process and system level. If this is an
        // expected exception, we handle it and return KERN_SUCCESS.
        bool handled = HandleMachException(request);
        kern_return_t replyCode = handled ? KERN_SUCCESS : KERN_FAILURE;

        // This magic incantation to send a reply back to the kernel was
        // derived from the exc_server generated by
        // 'mig -v /usr/include/mach/mach_exc.defs'.
        __Reply__exception_raise_t reply;
        reply.Head.msgh_bits = MACH_MSGH_BITS(MACH_MSGH_BITS_REMOTE(request.body.Head.msgh_bits), 0);
        reply.Head.msgh_size = sizeof(reply);
        reply.Head.msgh_remote_port = request.body.Head.msgh_remote_port;
        reply.Head.msgh_local_port = MACH_PORT_NULL;
        reply.Head.msgh_id = request.body.Head.msgh_id + 100;
        reply.NDR = NDR_record;
        reply.RetCode = replyCode;
        mach_msg(&reply.Head, MACH_SEND_MSG, sizeof(reply), 0, MACH_PORT_NULL,
                 MACH_MSG_TIMEOUT_NONE, MACH_PORT_NULL);
    }

    return nullptr;
}

#else  // If not Windows or Mac, assume Unix

static struct sigaction sPrevSIGSEGVHandler;
static struct sigaction sPrevSIGBUSHandler;
static struct sigaction sPrevSIGILLHandler;
static struct sigaction sPrevSIGFPEHandler;

static void
WasmTrapHandler(int signum, siginfo_t* info, void* context)
{
    if (!sAlreadyHandlingTrap) {
        AutoHandlingTrap aht;
        assert(signum == SIGSEGV || signum == SIGBUS || signum == SIGFPE || signum == SIGILL);

        void *JmpBuf = HandleTrap(ContextToPC(static_cast<CONTEXT*>(context)), signum, info, context);

        // Test if a custom instance signal handler handled the exception
        if (((size_t) JmpBuf) == 1)
            return;

        // Otherwise test if we need to longjmp to this buffer
        if (JmpBuf != nullptr) {
            sAlreadyHandlingTrap = false;
            Unwind(JmpBuf);
        }

        // ... and otherwise call the previous signal handler, if one is there
    }

    struct sigaction* previousSignal = nullptr;
    switch (signum) {
      case SIGSEGV: previousSignal = &sPrevSIGSEGVHandler; break;
      case SIGBUS: previousSignal = &sPrevSIGBUSHandler; break;
      case SIGFPE: previousSignal = &sPrevSIGFPEHandler; break;
      case SIGILL: previousSignal = &sPrevSIGILLHandler; break;
    }
    assert(previousSignal);

    // This signal is not for any compiled wasm code we expect, so we need to
    // forward the signal to the next handler. If there is no next handler (SIG_IGN
    // or SIG_DFL), then it's time to crash. To do this, we set the signal back to
    // its original disposition and return. This will cause the faulting op to
    // be re-executed which will crash in the normal way. The advantage of
    // doing this to calling _exit() is that we remove ourselves from the crash
    // stack which improves crash reports. If there is a next handler, call it.
    // It will either crash synchronously, fix up the instruction so that
    // execution can continue and return, or trigger a crash by returning the
    // signal to it's original disposition and returning.
    //
    // Note: the order of these tests matter.
    if (previousSignal->sa_flags & SA_SIGINFO) {
        previousSignal->sa_sigaction(signum, info, context);
    } else if (previousSignal->sa_handler == SIG_DFL || previousSignal->sa_handler == SIG_IGN) {
        sigaction(signum, previousSignal, nullptr);
    } else {
        previousSignal->sa_handler(signum);
    }
}
# endif // _WIN32 || __APPLE__ || assume unix

#if defined(ANDROID) && defined(MOZ_LINKER)
extern "C" MFBT_API bool IsSignalHandlingBroken();
#endif

int
EnsureEagerSignalHandlers()
{
#if defined(ANDROID) && defined(MOZ_LINKER)
    // Signal handling is broken on some android systems.
    if (IsSignalHandlingBroken()) {
        return false;
    }
#endif

    sAlreadyHandlingTrap = false;

    // Install whatever exception/signal handler is appropriate for the OS.
#if defined(_WIN32)

# if defined(MOZ_ASAN)
    // Under ASan we need to let the ASan runtime's ShadowExceptionHandler stay
    // in the first handler position. This requires some coordination with
    // MemoryProtectionExceptionHandler::isDisabled().
    const bool firstHandler = false;
# else
    // Otherwise, WasmTrapHandler needs to go first, so that we can recover
    // from wasm faults and continue execution without triggering handlers
    // such as MemoryProtectionExceptionHandler that assume we are crashing.
    const bool firstHandler = true;
# endif
    if (!AddVectoredExceptionHandler(firstHandler, WasmTrapHandler)) {
        // Windows has all sorts of random security knobs for disabling things
        // so make this a dynamic failure that disables wasm, not an abort().
        return false;
    }

#elif defined(USE_APPLE_MACH_PORTS)
    // All the Mach setup in EnsureDarwinMachPorts.
#else
    // SA_ONSTACK allows us to handle signals on an alternate stack, so that
    // the handler can run in response to running out of stack space on the
    // main stack. Rust installs an alternate stack with sigaltstack, so we
    // rely on that.

    // SA_NODEFER allows us to reenter the signal handler if we crash while
    // handling the signal, and fall through to the Breakpad handler by testing
    // handlingSegFault.

    // Allow handling OOB with signals on all architectures
    struct sigaction faultHandler;
    faultHandler.sa_flags = SA_SIGINFO | SA_NODEFER | SA_ONSTACK;
    faultHandler.sa_sigaction = WasmTrapHandler;
    sigemptyset(&faultHandler.sa_mask);
    if (sigaction(SIGSEGV, &faultHandler, &sPrevSIGSEGVHandler)) {
        perror("unable to install SIGSEGV handler");
        abort();
    }

# if defined(__arm__) || defined(__APPLE__)
    // On ARM, handle Unaligned Accesses.
    // On Darwin, guard page accesses are raised as SIGBUS.
    struct sigaction busHandler;
    busHandler.sa_flags = SA_SIGINFO | SA_NODEFER | SA_ONSTACK;
    busHandler.sa_sigaction = WasmTrapHandler;
    sigemptyset(&busHandler.sa_mask);
    if (sigaction(SIGBUS, &busHandler, &sPrevSIGBUSHandler)) {
        perror("unable to install SIGBUS handler");
        abort();
    }
# endif

# if !defined(__mips__)
    // Wasm traps for MIPS currently only raise integer overflow fp exception.
    struct sigaction illHandler;
    illHandler.sa_flags = SA_SIGINFO | SA_NODEFER | SA_ONSTACK;
    illHandler.sa_sigaction = WasmTrapHandler;
    sigemptyset(&illHandler.sa_mask);
    if (sigaction(SIGILL, &illHandler, &sPrevSIGILLHandler)) {
        perror("unable to install wasm SIGILL handler");
        abort();
    }
# endif

# if defined(__i386__) || defined(__x86_64__) || defined(__mips__)
    // x86 uses SIGFPE to report division by zero, and wasm traps for MIPS
    // currently raise integer overflow fp exception.
    struct sigaction fpeHandler;
    fpeHandler.sa_flags = SA_SIGINFO | SA_NODEFER | SA_ONSTACK;
    fpeHandler.sa_sigaction = WasmTrapHandler;
    sigemptyset(&fpeHandler.sa_mask);
    if (sigaction(SIGFPE, &fpeHandler, &sPrevSIGFPEHandler)) {
        perror("unable to install wasm SIGFPE handler");
        abort();
    }
# endif

#endif

    return true;
}
