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
;;       movq    0x10(%r10), %r10
;;       addq    $0x30, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x97
;;   19: subq    $0x20, %rsp
;;       movq    %rbx, (%rsp)
;;       movq    %r12, 8(%rsp)
;;       movq    %r13, 0x10(%rsp)
;;       movq    %r14, 0x18(%rsp)
;;       movq    %rdi, %rbx
;;       movq    %rcx, %r12
;;       movq    %rdx, %r14
;;       callq   0x3a9
;;       movq    %rax, %r13
;;       movl    $0x4000000, %esi
;;       movl    $3, %edx
;;       movl    $0x30, %ecx
;;       movl    $8, %r8d
;;       movq    %rbx, %rdi
;;       callq   0x335
;;       movq    8(%rbx), %rdx
;;       movq    0x18(%rdx), %rdx
;;       movl    %eax, %r8d
;;       movq    %r14, %rsi
;;       movl    %esi, 0x20(%rdx, %r8)
;;       movq    %r12, %rcx
;;       movq    %rcx, 0x28(%rdx, %r8)
;;       movq    %r13, %r9
;;       movl    %r9d, 0x18(%rdx, %r8)
;;       movl    $0, 0x1c(%rdx, %r8)
;;       movq    %rax, %rsi
;;       movq    %rbx, %rdi
;;       callq   0x3e5
;;       ud2
;;       ud2
;;
;; wasm[0]::function[1]::catch:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x10(%r10), %r10
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x12f
;;   b9: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdi, (%rsp)
;;       movq    (%rsp), %rsi
;;       movq    (%rsp), %rdi
;;       callq   0
;;       ╰─╼ exception handler: tag=0, context at [SP+0x0], handler=0xf6
;;       movl    $0x2a, %eax
;;       movl    $0x64, %ecx
;;       jmp     0x10d
;;   f6: movq    (%rsp), %rdi
;;       movq    8(%rdi), %rcx
;;       movq    0x18(%rcx), %rcx
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
;;  12f: ud2
