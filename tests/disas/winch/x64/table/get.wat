;;! target = "x86_64"
;;! test = "winch"
(module
  (table $t3 3 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
  )
)


;; wasm[0]::function[0]::dummy:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x38
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   38: ud2
;;
;; wasm[0]::function[1]::f3:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x10e
;;   5c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movq    0x40(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x110
;;   9e: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x38(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0xfe
;;   c4: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0x2db
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       jmp     0x105
;;   fe: andq    $0xfffffffffffffffe, %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  10e: ud2
;;  110: ud2
