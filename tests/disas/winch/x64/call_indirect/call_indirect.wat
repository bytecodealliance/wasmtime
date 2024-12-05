;;! target="x86_64"
;;! test = "winch"

(module
  (type $over-i32 (func (param i32) (result i32)))

  (table funcref
    (elem
      $fib-i32
    )
  )

  (func $fib-i32 (export "fib-i32") (type $over-i32)
    (if (result i32) (i32.le_u (local.get 0) (i32.const 1))
      (then (i32.const 1))
      (else
        (i32.add
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 2))
            (i32.const 0)
          )
          (call_indirect (type $over-i32)
            (i32.sub (local.get 0) (i32.const 1))
            (i32.const 0)
          )
        )
      )
    )
  )
)


;; wasm[0]::function[0]::fib-i32:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1df
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       cmpl    $1, %eax
;;       movl    $0, %eax
;;       setbe   %al
;;       testl   %eax, %eax
;;       je      0x53
;;   49: movl    $1, %eax
;;       jmp     0x1d9
;;   53: movl    0xc(%rsp), %eax
;;       subl    $2, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movq    0x60(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x1e1
;;   76: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xc7
;;   99: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    8(%rsp), %edx
;;       callq   0x31a
;;       addq    $8, %rsp
;;       addq    $4, %rsp
;;       movq    0x1c(%rsp), %r14
;;       jmp     0xcb
;;   c7: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x1e3
;;   d4: movq    0x50(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x1e5
;;   e6: pushq   %rax
;;       popq    %rcx
;;       movq    0x18(%rcx), %r8
;;       movq    8(%rcx), %rbx
;;       subq    $0xc, %rsp
;;       movq    %r8, %rdi
;;       movq    %r14, %rsi
;;       movl    0xc(%rsp), %edx
;;       callq   *%rbx
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       movl    0xc(%rsp), %ecx
;;       subl    $1, %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movq    0x60(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x1e7
;;  137: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x188
;;  15a: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    4(%rsp), %edx
;;       callq   0x31a
;;       addq    $4, %rsp
;;       addq    $4, %rsp
;;       movq    0x20(%rsp), %r14
;;       jmp     0x18c
;;  188: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x1e9
;;  195: movq    0x50(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x1eb
;;  1a7: pushq   %rax
;;       popq    %rcx
;;       movq    0x18(%rcx), %r8
;;       movq    8(%rcx), %rbx
;;       subq    $8, %rsp
;;       movq    %r8, %rdi
;;       movq    %r14, %rsi
;;       movl    8(%rsp), %edx
;;       callq   *%rbx
;;       addq    $8, %rsp
;;       addq    $4, %rsp
;;       movq    0x1c(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  1df: ud2
;;  1e1: ud2
;;  1e3: ud2
;;  1e5: ud2
;;  1e7: ud2
;;  1e9: ud2
;;  1eb: ud2
