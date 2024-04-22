;;! target = "x86_64"
;;! test = "winch"
(module
  (type $t0 (func))
  (func $f1 (type $t0))
  (func $f2 (type $t0))
  (func $f3 (type $t0))

  ;; Define two tables of funcref
  (table $t1 3 funcref)
  (table $t2 10 funcref)

  ;; Initialize table $t1 with functions $f1, $f2, $f3
  (elem (i32.const 0) $f1 $f2 $f3)

  ;; Function to fill table $t1 using a function reference from table $t2
  (func (export "fill") (param $i i32) (param $r i32) (param $n i32)
    (local $ref funcref)
    (local.set $ref (table.get $t1 (local.get $r)))
    (table.fill $t2 (local.get $i) (local.get $ref) (local.get $n))
  )
)
;; wasm[0]::function[0]:
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
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x71
;;   5b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   71: ud2
;;
;; wasm[0]::function[2]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb1
;;   9b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   b1: ud2
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x1d8
;;   db: movq    %rdi, %r14
;;       subq    $0x28, %rsp
;;       movq    %rdi, 0x20(%rsp)
;;       movq    %rsi, 0x18(%rsp)
;;       movl    %edx, 0x14(%rsp)
;;       movl    %ecx, 0x10(%rsp)
;;       movl    %r8d, 0xc(%rsp)
;;       movl    $0, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movl    0x10(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movl    0x60(%rdx), %ebx
;;       cmpl    %ebx, %ecx
;;       jae     0x1da
;;  12b: movl    %ecx, %r11d
;;       imulq   $8, %r11, %r11
;;       movq    0x58(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpl    %ebx, %ecx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x17c
;;  14e: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    4(%rsp), %edx
;;       callq   0x5f4
;;       addq    $4, %rsp
;;       addq    $4, %rsp
;;       movq    0x20(%rsp), %r14
;;       jmp     0x180
;;  17c: andq    $0xfffffffffffffffe, %rax
;;       movq    %rax, 4(%rsp)
;;       movl    0x14(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       movl    0x18(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movl    $1, %esi
;;       movl    0x14(%rsp), %edx
;;       movq    0xc(%rsp), %rcx
;;       movl    8(%rsp), %r8d
;;       callq   0x636
;;       addq    $8, %rsp
;;       addq    $0x10, %rsp
;;       movq    0x20(%rsp), %r14
;;       addq    $0x28, %rsp
;;       popq    %rbp
;;       retq
;;  1d8: ud2
;;  1da: ud2
