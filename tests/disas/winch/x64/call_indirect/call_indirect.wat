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
;;       movq    (%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1c8
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       cmpl    $1, %eax
;;       movl    $0, %eax
;;       setbe   %al
;;       testl   %eax, %eax
;;       je      0x52
;;   48: movl    $1, %eax
;;       jmp     0x1c2
;;   52: movl    4(%rsp), %eax
;;       subl    $2, %eax
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movl    0x60(%rdx), %ebx
;;       cmpl    %ebx, %ecx
;;       jae     0x1ca
;;   73: movl    %ecx, %r11d
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xbb
;;   96: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    (%rsp), %edx
;;       callq   0x2dd
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       jmp     0xbf
;;   bb: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x1cc
;;   c8: movq    0x50(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x1ce
;;   da: pushq   %rax
;;       popq    %rcx
;;       movq    0x18(%rcx), %r8
;;       movq    8(%rcx), %rbx
;;       subq    $4, %rsp
;;       movq    %r8, %rdi
;;       movq    %r14, %rsi
;;       movl    4(%rsp), %edx
;;       callq   *%rbx
;;       addq    $4, %rsp
;;       addq    $4, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    4(%rsp), %ecx
;;       subl    $1, %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       movl    $0, %ecx
;;       movq    %r14, %rdx
;;       movl    0x60(%rdx), %ebx
;;       cmpl    %ebx, %ecx
;;       jae     0x1d0
;;  129: movl    %ecx, %r11d
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x17a
;;  14c: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0x2dd
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0x17e
;;  17a: andq    $0xfffffffffffffffe, %rax
;;       testq   %rax, %rax
;;       je      0x1d2
;;  187: movq    0x50(%r14), %r11
;;       movl    (%r11), %ecx
;;       movl    0x10(%rax), %edx
;;       cmpl    %edx, %ecx
;;       jne     0x1d4
;;  199: pushq   %rax
;;       popq    %rcx
;;       movq    0x18(%rcx), %r8
;;       movq    8(%rcx), %rbx
;;       movq    %r8, %rdi
;;       movq    %r14, %rsi
;;       movl    (%rsp), %edx
;;       callq   *%rbx
;;       addq    $4, %rsp
;;       movq    0x14(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       addl    %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;  1c8: ud2
;;  1ca: ud2
;;  1cc: ud2
;;  1ce: ud2
;;  1d0: ud2
;;  1d2: ud2
;;  1d4: ud2
