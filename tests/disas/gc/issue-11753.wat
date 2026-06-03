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
;;       ja      0x11a
;;   19: subq    $0x40, %rsp
;;       movq    %rbx, 0x10(%rsp)
;;       movq    %r12, 0x18(%rsp)
;;       movq    %r13, 0x20(%rsp)
;;       movq    %r14, 0x28(%rsp)
;;       movq    %r15, 0x30(%rsp)
;;       movq    0x20(%rdi), %rsi
;;       movl    (%rsi), %eax
;;       movl    %eax, %ecx
;;       leaq    0x20(%rcx), %rdx
;;       movl    4(%rsi), %r8d
;;       cmpq    %r8, %rdx
;;       ja      0xe6
;;   4f: movq    %rdi, 8(%rsp)
;;       leal    0x20(%rax), %edi
;;       movl    %edi, (%rsi)
;;       movq    8(%rsp), %r9
;;       movq    8(%r9), %rsi
;;       movq    0x20(%rsi), %rdi
;;       leaq    (%rdi, %rcx), %rsi
;;       movl    $0xb0000002, (%rdi, %rcx)
;;       movq    0x28(%r9), %r8
;;       movl    (%r8), %r8d
;;       movl    %r8d, 4(%rdi, %rcx)
;;       movl    $0x20, %r8d
;;       movl    %r8d, 8(%rdi, %rcx)
;;       movl    %eax, (%rsp)
;;       movl    $0x2a, 0x10(%rsi)
;;       ╰─╼ trap: GcHeapCorrupt
;;       movq    8(%rsp), %r9
;;       movq    0x38(%r9), %rax
;;       movq    0x48(%r9), %rdi
;;       movq    8(%rsp), %rsi
;;       callq   *%rax
;;       ├─╼ exception frame offset: SP = FP - 0x40
;;       ╰─╼ exception handler: default handler, context at [SP+0x8], handler=0xa6
;;       movl    (%rsp), %eax
;;       ╰─╼ stack_map: frame_size=64, frame_offsets=[0]
;;       testl   %eax, %eax
;;       je      0x11c
;;   b1: movq    8(%rsp), %r9
;;       movq    8(%r9), %rcx
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
;;   e6: movl    $0xb0000002, %esi
;;   eb: movq    0x28(%rdi), %rax
;;   ef: movq    %rdi, 8(%rsp)
;;   f4: movl    (%rax), %edx
;;   f6: movl    $0x20, %ecx
;;   fb: movl    $0x10, %r8d
;;  101: callq   0x271
;;  106: movq    8(%rsp), %r9
;;  10b: movq    8(%r9), %rcx
;;  10f: movl    %eax, %esi
;;  111: addq    0x20(%rcx), %rsi
;;  115: jmp     0x88
;;  11a: ud2
;;       ╰─╼ trap: Normal(StackOverflow)
;;  11c: ud2
;;       ╰─╼ trap: Normal(NullReference)
