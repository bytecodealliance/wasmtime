;;! target = "x86_64"
;;! test = 'compile'
;;! filter = "function"
;;! flags = "-C inlining=n -Wconcurrency-support=n"

;; Same as `direct-adapter-calls.wat` but shows full compilation down to x86_64
;; so that we can exercise our linker's ability to resolve relocations for
;; direct calls across Wasm modules.

(component
  (component $A
    (core module $M
      (func (export "f'") (param i32) (result i32)
        (i32.add (local.get 0) (i32.const 42))
      )
    )

    (core instance $m (instantiate $M))

    (func (export "f") (param "x" u32) (result u32)
      (canon lift (core func $m "f'"))
    )
  )

  (component $B
    (import "f" (func $f (param "x" u32) (result u32)))

    (core func $f' (canon lower (func $f)))

    (core module $N
      (import "" "f'" (func $f' (param i32) (result i32)))
      (func (export "g'") (result i32)
        (call $f' (i32.const 1234))
      )
    )

    (core instance $n
      (instantiate $N
        (with "" (instance (export "f'" (func $f'))))
      )
    )

    (func (export "g") (result u32)
      (canon lift (core func $n "g'"))
    )
  )

  (instance $a (instantiate $A))
  (instance $b
    (instantiate $B
      (with "f" (func $a "f"))
    )
  )

  (export "g" (func $b "g"))
)

;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       leal    0x2a(%rdx), %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[1]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x10, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x4f
;;   39: movq    %rdi, %rsi
;;       movq    0x48(%rdi), %rdi
;;       movl    $0x4d2, %edx
;;       callq   0x60
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;
;; wasm[2]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x60, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x140
;;   79: subq    $0x50, %rsp
;;       movq    %rbx, 0x20(%rsp)
;;       movq    %r12, 0x28(%rsp)
;;       movq    %r13, 0x30(%rsp)
;;       movq    %r14, 0x38(%rsp)
;;       movq    %r15, 0x40(%rsp)
;;       movq    %rdi, (%rsp)
;;       movq    (%rsp), %rdi
;;       movq    0x88(%rdi), %rcx
;;       movl    (%rcx), %eax
;;       movq    %rcx, 0x10(%rsp)
;;       testl   %eax, %eax
;;       movq    %rax, 8(%rsp)
;;       jne     0xd5
;;   b9: movq    (%rsp), %rdi
;;       movq    0x58(%rdi), %rax
;;       movq    0x68(%rdi), %rdi
;;       movl    $0x17, %edx
;;       movq    (%rsp), %rsi
;;       callq   *%rax
;;       ├─╼ exception frame offset: SP = FP - 0x50
;;       ╰─╼ exception handler: default handler, context at [SP+0x0], handler=0x12b
;;       jmp     0x129
;;   d5: movq    (%rsp), %rsi
;;       movq    0x70(%rsi), %rax
;;       movl    (%rax), %ecx
;;       movl    $0, (%rax)
;;       movl    %ecx, (%rax)
;;       movq    0x48(%rsi), %rdi
;;       callq   0
;;       ├─╼ exception frame offset: SP = FP - 0x50
;;       ╰─╼ exception handler: default handler, context at [SP+0x0], handler=0x12b
;;       movq    0x10(%rsp), %rcx
;;       movl    $0, (%rcx)
;;       movq    8(%rsp), %rdx
;;       movl    %edx, (%rcx)
;;       movq    0x20(%rsp), %rbx
;;       movq    0x28(%rsp), %r12
;;       movq    0x30(%rsp), %r13
;;       movq    0x38(%rsp), %r14
;;       movq    0x40(%rsp), %r15
;;       addq    $0x50, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  124: jmp     0x12b
;;  129: ud2
;;  12b: movq    (%rsp), %rsi
;;  12f: movq    0x58(%rsi), %rax
;;  133: movq    0x68(%rsi), %rdi
;;  137: movl    $0x31, %edx
;;  13c: callq   *%rax
;;  13e: ud2
;;  140: ud2
