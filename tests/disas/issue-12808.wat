;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Dguest-debug=yes", "-Wthreads=yes"]

(module
  (memory 1 1 shared)
  (func (result i32 i32)
    i32.const 0
    i32.const 0
    i32.const 0
    i32.const 0
    i32.atomic.rmw.cmpxchg))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       subq    $0x30, %rsp
;;       movq    %r14, 0x20(%rsp)
;;       movq    8(%rdi), %rax
;;       movq    0x18(%rax), %rax
;;       movq    %rsp, %rcx
;;       cmpq    %rax, %rcx
;;       jb      0x93
;;   21: movq    %rdi, (%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x1f, slot at FP-0x30, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 0x1f, patch bytes [232, 227, 1, 0, 0]
;;       movl    $0, 8(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x21, slot at FP-0x30, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 0x21, patch bytes [232, 214, 1, 0, 0]
;;       movl    $0, 0xc(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x23, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc
;;       ╰─╼ breakpoint patch: wasm PC 0x23, patch bytes [232, 201, 1, 0, 0]
;;       movl    $0, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x25, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc, I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 0x25, patch bytes [232, 188, 1, 0, 0]
;;       movl    $0, 0x14(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x27, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc, I32 @ slot+0x10, I32 @ slot+0x14
;;       ╰─╼ breakpoint patch: wasm PC 0x27, patch bytes [232, 175, 1, 0, 0]
;;       movq    0x30(%rdi), %rax
;;       movq    (%rax), %rcx
;;       xorl    %eax, %eax
;;       lock cmpxchgl %eax, (%rcx)
;;       movl    %eax, 0xc(%rsp)
;;       movq    %rax, %rcx
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x2b, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc
;;       ╰─╼ breakpoint patch: wasm PC 0x2b, patch bytes [232, 150, 1, 0, 0]
;;       xorl    %eax, %eax
;;       movl    $0, 8(%rsp)
;;       movl    %ecx, 0xc(%rsp)
;;       movq    0x20(%rsp), %r14
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   93: movq    %rdi, %r14
;;   96: xorl    %esi, %esi
;;   98: callq   0x1af
;;   9d: movq    %r14, %rdi
;;   a0: callq   0x1e0
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 0x1e, slot at FP-0x30, locals , stack 
;;   a5: ud2
