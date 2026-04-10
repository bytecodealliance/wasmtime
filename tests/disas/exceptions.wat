;;! target = "x86_64"
;;! test = "compile"
;;! flags = ["-Wexceptions=yes", "-Wgc=yes"]

(module
 (tag $e0 (param i32 i64))

 (func $throw (param i32 i64)
       (throw $e0 (local.get 0) (local.get 1)))

 (func $catch (export "catch") (param i32 i64) (result i32 i64)

       (block $b (result i32 i64)
              (try_table (result i32 i64)
                         (catch $e0 $b)
                         (call $throw (local.get 0) (local.get 1))
                         (i32.const 42)
                         (i64.const 100)))))
;; wasm[0]::function[0]::throw:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x9f
;;   19: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdi, %rbx
;;       movq    %rcx, %r13
;;       movq    %rdx, %r14
;;       callq   0x3a5
;;       movq    %rax, %r12
;;       movl    $0x4000000, %esi
;;       movq    0x28(%rbx), %rax
;;       movl    0xc(%rax), %edx
;;       movl    $0x30, %ecx
;;       movl    $8, %r8d
;;       movq    %rbx, %rdi
;;       callq   0x342
;;       movq    8(%rbx), %rcx
;;       movq    0x20(%rcx), %rcx
;;       movl    %eax, %edx
;;       movq    %r14, %rsi
;;       movl    %esi, 0x20(%rcx, %rdx)
;;       movq    %r13, %rsi
;;       movq    %rsi, 0x28(%rcx, %rdx)
;;       movq    %r12, %rsi
;;       movl    %esi, 0x18(%rcx, %rdx)
;;       movl    $0, 0x1c(%rcx, %rdx)
;;       movq    %rax, %rsi
;;       movq    %rbx, %rdi
;;       movq    %rbx, (%rsp)
;;       callq   0x3d2
;;       ud2
;;       ud2
;;
;; wasm[0]::function[1]::catch:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x155
;;   d9: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdi, (%rsp)
;;       movq    (%rsp), %rsi
;;       movq    (%rsp), %rdi
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: tag=0, context at [SP+0x0], handler=0x116
;;       movl    $0x2a, %eax
;;       movl    $0x64, %ecx
;;       jmp     0x133
;;  116: movq    %rax, %rdx
;;       movq    (%rsp), %rsi
;;       movq    8(%rsi), %rax
;;       movq    0x20(%rax), %rcx
;;       movq    %rdx, %rax
;;       movl    %eax, %edx
;;       movl    0x20(%rcx, %rdx), %eax
;;       movq    0x28(%rcx, %rdx), %rcx
;;       movq    0x10(%rsp), %rbx
;;       movq    0x18(%rsp), %r12
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  155: ud2
