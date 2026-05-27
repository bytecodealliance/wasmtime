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
;;       ja      0xe7
;;   19: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdi, %rbx
;;       movq    %rcx, %r13
;;       movq    %rdx, %r14
;;       callq   0x3e5
;;       movq    %rax, %r12
;;       movq    0x20(%rbx), %rdx
;;       movl    (%rdx), %esi
;;       movl    %esi, %ecx
;;       leaq    0x30(%rcx), %rax
;;       movl    4(%rdx), %edi
;;       cmpq    %rdi, %rax
;;       ja      0xb3
;;   5f: leal    0x30(%rsi), %eax
;;       movl    %eax, (%rdx)
;;       movq    8(%rbx), %rax
;;       movq    0x20(%rax), %rdi
;;       leaq    (%rdi, %rcx), %rdx
;;       movl    $0x4000002, (%rdi, %rcx)
;;       movq    0x28(%rbx), %rax
;;       movl    0xc(%rax), %eax
;;       movl    %eax, 4(%rdi, %rcx)
;;       movl    $0x30, %eax
;;       movl    %eax, 8(%rdi, %rcx)
;;       movq    %r14, %rdi
;;       movl    %edi, 0x18(%rdx)
;;       movq    %r13, %rcx
;;       movq    %rcx, 0x20(%rdx)
;;       movq    %r12, %rax
;;       movl    %eax, 0x10(%rdx)
;;       movl    $0, 0x14(%rdx)
;;       movq    %rbx, %rdi
;;       movq    %rbx, (%rsp)
;;       callq   0x412
;;       ud2
;;       movl    $0x4000002, %esi
;;       movq    0x28(%rbx), %rax
;;       movl    0xc(%rax), %edx
;;       movl    $0x30, %ecx
;;       movl    $0x10, %r8d
;;       movq    %rbx, %rdi
;;       callq   0x382
;;       movq    8(%rbx), %rcx
;;       movl    %eax, %edx
;;       addq    0x20(%rcx), %rdx
;;       movq    %rax, %rsi
;;       movq    %r14, %rdi
;;       jmp     0x8e
;;   e7: ud2
;;
;; wasm[0]::function[1]::catch:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x195
;;  119: subq    $0x40, %rsp
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
;;       ╰─╼ exception handler: tag=0, context at [SP+0x0], handler=0x156
;;       movl    $0x2a, %eax
;;       movl    $0x64, %ecx
;;       jmp     0x173
;;  156: movq    %rax, %rdx
;;       movq    (%rsp), %rsi
;;       movq    8(%rsi), %rax
;;       movq    0x20(%rax), %rcx
;;       movq    %rdx, %rax
;;       movl    %eax, %edx
;;       movl    0x18(%rcx, %rdx), %eax
;;       movq    0x20(%rcx, %rdx), %rcx
;;       movq    0x10(%rsp), %rbx
;;       movq    0x18(%rsp), %r12
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  195: ud2
