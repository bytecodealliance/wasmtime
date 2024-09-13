;;! target = "x86_64"
;;! test = "winch"


(module
  (table $t3 2 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)

  (func (export "set-funcref") (param $i i32) (param $r funcref)
    (table.set $t3 (local.get $i) (local.get $r))
  )
  (func (export "set-funcref-from") (param $i i32) (param $j i32)
    (table.set $t3 (local.get $i) (table.get $t3 (local.get $j)))
  )
)

;; wasm[0]::function[0]::dummy:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x31
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   31: ud2
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb0
;;   5b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movq    %rcx, (%rsp)
;;       movq    (%rsp), %rax
;;       movl    0xc(%rsp), %ecx
;;       movq    %r14, %rdx
;;       movq    0x60(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0xb2
;;   8c: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       orq     $1, %rax
;;       movq    %rax, (%rdx)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   b0: ud2
;;   b2: ud2
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1b5
;;   db: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    0xc(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movq    0x60(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x1b7
;;  125: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x176
;;  148: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    8(%rsp), %edx
;;       callq   0x37e
;;       addq    $8, %rsp
;;       addq    $4, %rsp
;;       movq    0x1c(%rsp), %r14
;;       jmp     0x17a
;;  176: andq    $0xfffffffffffffffe, %rax
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movq    0x60(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x1b9
;;  191: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       orq     $1, %rax
;;       movq    %rax, (%rdx)
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  1b5: ud2
;;  1b7: ud2
;;  1b9: ud2
