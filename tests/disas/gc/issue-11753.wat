;;! target = "x86_64"
;;! flags = "-W gc,exceptions"
;;! test = "compile"
;;! objdump = "--stack-maps=true"

(module
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
;;       addq    $0x50, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x118
;;   19: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    %rdi, 8(%rsp)
;;       movq    0x20(%rdi), %rdx
;;       movl    (%rdx), %eax
;;       movl    %eax, %ecx
;;       leaq    0x20(%rcx), %rsi
;;       movl    4(%rdx), %r8d
;;       cmpq    %r8, %rsi
;;       ja      0xe6
;;   54: leal    0x20(%rax), %esi
;;       movl    %esi, (%rdx)
;;       movq    8(%rsp), %rdi
;;       movq    8(%rdi), %rdx
;;       movq    0x20(%rdx), %rsi
;;       leaq    (%rsi, %rcx), %rdx
;;       movl    $0xb0000002, (%rsi, %rcx)
;;       ╰─╼ trap: GcHeapCorrupt
;;       movq    0x28(%rdi), %r8
;;       movl    (%r8), %r8d
;;       movl    %r8d, 4(%rsi, %rcx)
;;       ╰─╼ trap: GcHeapCorrupt
;;       movl    $0x20, %r8d
;;       movl    %r8d, 8(%rsi, %rcx)
;;       ╰─╼ trap: GcHeapCorrupt
;;       movl    %eax, (%rsp)
;;       movl    $0x2a, 0x10(%rdx)
;;       ╰─╼ trap: GcHeapCorrupt
;;       movq    8(%rsp), %rdi
;;       movq    0x38(%rdi), %rax
;;       movq    0x48(%rdi), %rdi
;;       movq    8(%rsp), %rsi
;;       callq   *%rax
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, context at [SP+0x8], handler=0xa6
;;       movl    (%rsp), %eax
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[0]
;;       testl   %eax, %eax
;;       je      0x11a
;;   b1: movq    8(%rsp), %rdi
;;       movq    8(%rdi), %rcx
;;       movq    0x20(%rcx), %rcx
;;       movl    %eax, %eax
;;       movl    0x10(%rcx, %rax), %eax
;;       ╰─╼ trap: GcHeapCorrupt
;;       movq    0x10(%rsp), %rbx
;;       movq    0x18(%rsp), %r12
;;       movq    0x20(%rsp), %r13
;;       movq    0x28(%rsp), %r14
;;       movq    0x30(%rsp), %r15
;;       addq    $0x40, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   e6: movq    8(%rsp), %rbx
;;   eb: movl    $0xb0000002, %esi
;;   f0: movq    0x28(%rbx), %rax
;;   f4: movl    (%rax), %edx
;;   f6: movl    $0x20, %ecx
;;   fb: movl    $0x10, %r8d
;;  101: movq    %rbx, %rdi
;;  104: callq   0x29a
;;  109: movq    8(%rbx), %rcx
;;  10d: movl    %eax, %edx
;;  10f: addq    0x20(%rcx), %rdx
;;  113: jmp     0x88
;;  118: ud2
;;       ╰─╼ trap: Normal(StackOverflow)
;;  11a: ud2
;;       ╰─╼ trap: Normal(NullReference)
