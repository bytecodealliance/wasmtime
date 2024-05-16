;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "main") (param i32) (param i32) (result i32)
    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))

    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))
  )

  (func $add (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add)
    (local.get 2)
    (i32.add)
    (local.get 3)
    (i32.add)
    (local.get 4)
    (i32.add)
    (local.get 5)
    (i32.add)
    (local.get 6)
    (i32.add)
    (local.get 7)
    (i32.add)
    (local.get 8)
    (i32.add)
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x50, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x152
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    4(%rsp), %ecx
;;       movl    (%rsp), %eax
;;       xorl    %edx, %edx
;;       divl    %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0x34, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    0x34(%rsp), %edx
;;       movl    $1, %ecx
;;       movl    $2, %r8d
;;       movl    $3, %r9d
;;       movl    $4, %r11d
;;       movl    %r11d, (%rsp)
;;       movl    $5, %r11d
;;       movl    %r11d, 8(%rsp)
;;       movl    $6, %r11d
;;       movl    %r11d, 0x10(%rsp)
;;       movl    $7, %r11d
;;       movl    %r11d, 0x18(%rsp)
;;       movl    $8, %r11d
;;       movl    %r11d, 0x20(%rsp)
;;       callq   0x160
;;       addq    $0x34, %rsp
;;       addq    $4, %rsp
;;       movq    0x10(%rsp), %r14
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movl    4(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movl    (%rsp), %eax
;;       addq    $4, %rsp
;;       xorl    %edx, %edx
;;       divl    %ecx
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       subq    $0x30, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       movl    0x34(%rsp), %edx
;;       movl    0x30(%rsp), %ecx
;;       movl    $2, %r8d
;;       movl    $3, %r9d
;;       movl    $4, %r11d
;;       movl    %r11d, (%rsp)
;;       movl    $5, %r11d
;;       movl    %r11d, 8(%rsp)
;;       movl    $6, %r11d
;;       movl    %r11d, 0x10(%rsp)
;;       movl    $7, %r11d
;;       movl    %r11d, 0x18(%rsp)
;;       movl    $8, %r11d
;;       movl    %r11d, 0x20(%rsp)
;;       callq   0x160
;;       addq    $0x30, %rsp
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;  152: ud2
;;
;; wasm[0]::function[1]::add:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1d3
;;  17b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    %r8d, 4(%rsp)
;;       movl    %r9d, (%rsp)
;;       movl    8(%rsp), %eax
;;       movl    0xc(%rsp), %ecx
;;       addl    %eax, %ecx
;;       movl    4(%rsp), %eax
;;       addl    %eax, %ecx
;;       movl    (%rsp), %eax
;;       addl    %eax, %ecx
;;       movl    0x10(%rbp), %eax
;;       addl    %eax, %ecx
;;       movl    0x18(%rbp), %eax
;;       addl    %eax, %ecx
;;       movl    0x20(%rbp), %eax
;;       addl    %eax, %ecx
;;       movl    0x28(%rbp), %eax
;;       addl    %eax, %ecx
;;       movl    0x30(%rbp), %eax
;;       addl    %eax, %ecx
;;       movl    %ecx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  1d3: ud2
