;;! target = "x86_64"
;;! test = "compile"
;;! filter = "wasm[1]"
;;! flags = "-C inlining=y -Wconcurrency-support=y"

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

;; wasm[1]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r10
;;       movq    0x18(%r10), %r10
;;       addq    $0x70, %r10
;;       cmpq    %rsp, %r10
;;       ja      0x156
;;   39: subq    $0x60, %rsp
;;       movq    %rbx, 0x30(%rsp)
;;       movq    %r12, 0x38(%rsp)
;;       movq    %r13, 0x40(%rsp)
;;       movq    %r14, 0x48(%rsp)
;;       movq    %r15, 0x50(%rsp)
;;       movq    0x48(%rdi), %rsi
;;       movq    0xc8(%rsi), %rax
;;       movl    (%rax), %ecx
;;       testl   %ecx, %ecx
;;       jne     0x84
;;   6b: movq    0x58(%rsi), %rax
;;       movq    0x68(%rsi), %rdi
;;       movq    %rsi, 0x20(%rsp)
;;       movl    $0x17, %edx
;;       callq   *%rax
;;       ├─╼ exception frame offset: SP = FP - 0x60
;;       ╰─╼ exception handler: default handler, context at [SP+0x20], handler=0x140
;;       jmp     0x13e
;;   84: movq    0xe0(%rsi), %rdx
;;       movl    (%rdx), %r8d
;;       movl    $0, (%rdx)
;;       movq    8(%rsi), %rdi
;;       movq    0x88(%rdi), %r9
;;       leaq    (%rsp), %rbx
;;       movq    %r9, (%rsp)
;;       movl    $2, 8(%rsp)
;;       movl    $0, 0xc(%rsp)
;;       movl    $1, 0x10(%rsp)
;;       movl    0x80(%rdi), %r10d
;;       movl    %r10d, 0x14(%rsp)
;;       movl    $0, 0x80(%rdi)
;;       movl    0x84(%rdi), %r11d
;;       movl    %r11d, 0x18(%rsp)
;;       movl    $0, 0x84(%rdi)
;;       movq    %rbx, 0x88(%rdi)
;;       movq    0xb0(%rsi), %rsi
;;       movl    (%rsi), %ebx
;;       movl    %ebx, (%rsi)
;;       movq    %r9, 0x88(%rdi)
;;       movl    %r10d, 0x80(%rdi)
;;       movl    %r11d, 0x84(%rdi)
;;       movl    %ecx, (%rax)
;;       movl    %r8d, (%rdx)
;;       movl    $0x4fc, %eax
;;       movq    0x30(%rsp), %rbx
;;       movq    0x38(%rsp), %r12
;;       movq    0x40(%rsp), %r13
;;       movq    0x48(%rsp), %r14
;;       movq    0x50(%rsp), %r15
;;       addq    $0x60, %rsp
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;  13e: ud2
;;  140: movq    0x20(%rsp), %rsi
;;  145: movq    0x58(%rsi), %rax
;;  149: movq    0x68(%rsi), %rdi
;;  14d: movl    $0x31, %edx
;;  152: callq   *%rax
;;  154: ud2
;;  156: ud2
