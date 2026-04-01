;;! target = "x86_64"
;;! flags = "-W gc,exceptions"
;;! test = "compile"

(module
  (type $a (array (mut i8)))
  (type $s (struct (field (mut i32))))

  (import "" "gc" (func $gc))

  (func (export "run") (result i32)
    (struct.new $s (i32.const 42))

    block $b
      try_table (catch_all $b)
        ;; This should have both exception handlers and stack maps in the
        ;; disassembly below.
        call $gc
      end
    end

    struct.get $s 0
  )
)

;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x60, %r10
;;       cmpq    %rsp, %r10
;;       ja      0xc1
;;   19: subq    $0x50, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %r12, 0x28(%rsp)
;;       movq    %r13, 0x30(%rsp)
;;       movq    %r14, 0x38(%rsp)
;;       movq    %r15, 0x40(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movl    $0xb0000000, %esi
;;       movl    $1, %edx
;;       movl    $0x20, %ecx
;;       movl    $8, %r8d
;;       movq    8(%rsp), %rbx
;;       movq    %rbx, %rdi
;;       callq   0x218
;;       movl    %eax, (%rsp)
;;       movq    8(%rbx), %rcx
;;       movq    0x20(%rcx), %rcx
;;       movl    %eax, %eax
;;       movl    $0x2a, 0x18(%rcx, %rax)
;;       movq    %rcx, 0x10(%rsp)
;;       movq    0x30(%rbx), %rax
;;       movq    0x40(%rbx), %rdi
;;       movq    %rbx, %rsi
;;       movq    %rbx, 8(%rsp)
;;       callq   *%rax
;;       ├─╼ exception frame offset: SP = FP - 0x50
;;       ╰─╼ exception handler: default handler, context at [SP+0x8], handler=0x89
;;       movl    (%rsp), %eax
;;       testl   %eax, %eax
;;       je      0xc3
;;   94: movl    %eax, %eax
;;       movq    0x10(%rsp), %rcx
;;       movl    0x18(%rcx, %rax), %eax
;;       movq    0x20(%rsp), %rbx
;;       movq    0x28(%rsp), %r12
;;       movq    0x30(%rsp), %r13
;;       movq    0x38(%rsp), %r14
;;       movq    0x40(%rsp), %r15
;;       addq    $0x50, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   c1: ud2
;;   c3: ud2
