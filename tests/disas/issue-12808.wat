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
;;       movq    %rbx, 0x20(%rsp)
;;       movq    8(%rdi), %rax
;;       movq    0x18(%rax), %rax
;;       movq    %rsp, %rcx
;;       cmpq    %rax, %rcx
;;       jb      0xb4
;;   21: movq    %rdi, (%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 31, slot at FP-0x30, locals , stack 
;;       ╰─╼ breakpoint patch: wasm PC 31, patch bytes [232, 4, 2, 0, 0]
;;       movl    $0, 8(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 33, slot at FP-0x30, locals , stack I32 @ slot+0x8
;;       ╰─╼ breakpoint patch: wasm PC 33, patch bytes [232, 247, 1, 0, 0]
;;       movl    $0, 0xc(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 35, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc
;;       ╰─╼ breakpoint patch: wasm PC 35, patch bytes [232, 234, 1, 0, 0]
;;       movl    $0, 0x10(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 37, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc, I32 @ slot+0x10
;;       ╰─╼ breakpoint patch: wasm PC 37, patch bytes [232, 221, 1, 0, 0]
;;       movl    $0, 0x14(%rsp)
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 39, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc, I32 @ slot+0x10, I32 @ slot+0x14
;;       ╰─╼ breakpoint patch: wasm PC 39, patch bytes [232, 208, 1, 0, 0]
;;       xorl    %eax, %eax
;;       testb   %al, %al
;;       jne     0x9d
;;   68: movq    0x30(%rdi), %rax
;;       movq    (%rax), %rcx
;;       xorl    %eax, %eax
;;       lock cmpxchgl %eax, (%rcx)
;;       movl    %eax, 0xc(%rsp)
;;       movq    %rax, %rcx
;;       nopl    (%rax, %rax)
;;       ├─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 43, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc
;;       ╰─╼ breakpoint patch: wasm PC 43, patch bytes [232, 173, 1, 0, 0]
;;       xorl    %eax, %eax
;;       movl    $0, 8(%rsp)
;;       movl    %ecx, 0xc(%rsp)
;;       movq    0x20(%rsp), %rbx
;;       addq    $0x30, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   9d: movq    %rdi, %rbx
;;   a0: movl    $2, %esi
;;   a5: callq   0x1d0
;;   aa: movq    %rbx, %rdi
;;   ad: callq   0x201
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 39, slot at FP-0x30, locals , stack I32 @ slot+0x8, I32 @ slot+0xc
;;   b2: ud2
;;   b4: movq    %rdi, %rbx
;;   b7: xorl    %esi, %esi
;;   b9: callq   0x1d0
;;   be: movq    %rbx, %rdi
;;   c1: callq   0x201
;;       ╰─╼ debug frame state (after previous inst): func key DefinedWasmFunction(StaticModuleIndex(0), DefinedFuncIndex(0)), wasm PC 30, slot at FP-0x30, locals , stack 
;;   c6: ud2
