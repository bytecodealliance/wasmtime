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
;; wasm[0]::function[0]::f1:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
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
;; wasm[0]::function[1]::f2:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x78
;;   5c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   78: ud2
;;
;; wasm[0]::function[2]::f3:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0xb8
;;   9c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   b8: ud2
;;
;; wasm[0]::function[3]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x40, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x22a
;;   dc: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movl    %edx, 0x1c(%rsp)
;;       movl    %ecx, 0x18(%rsp)
;;       movl    %r8d, 0x14(%rsp)
;;       movl    $0, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movl    0x18(%rsp), %r11d
;;       subq    $4, %rsp
;;       movl    %r11d, (%rsp)
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       movq    %r14, %rdx
;;       movq    0x38(%rdx), %rbx
;;       cmpq    %rbx, %rcx
;;       jae     0x22c
;;  138: movq    %rcx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x30(%rdx), %rdx
;;       movq    %rdx, %rsi
;;       addq    %r11, %rdx
;;       cmpq    %rbx, %rcx
;;       cmovaeq %rsi, %rdx
;;       movq    (%rdx), %rax
;;       testq   %rax, %rax
;;       jne     0x199
;;  15f: subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $0xc, %rsp
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    0xc(%rsp), %edx
;;       callq   0x51d
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x28(%rsp), %r14
;;       jmp     0x19f
;;  199: andq    $0xfffffffffffffffe, %rax
;;       movq    %rax, 0xc(%rsp)
;;       movl    0x14(%rsp), %eax
;;       movq    0xc(%rsp), %rcx
;;       movl    0x1c(%rsp), %edx
;;       movq    %r14, %r11
;;       movq    0x48(%r11), %rbx
;;       movl    %edx, %esi
;;       addl    %eax, %esi
;;       jb      0x22e
;;  1c2: cmpl    %ebx, %esi
;;       ja      0x230
;;  1ca: cmpq    $0, %rax
;;       je      0x221
;;  1d4: movq    %rdx, %rbx
;;       movq    %rcx, %rsi
;;       movq    %r14, %rdi
;;       movq    0x48(%rdi), %r8
;;       cmpq    %r8, %rbx
;;       jae     0x232
;;  1ea: movq    %rbx, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x40(%rdi), %rdi
;;       movq    %rdi, %r9
;;       addq    %r11, %rdi
;;       cmpq    %r8, %rbx
;;       cmovaeq %r9, %rdi
;;       orq     $1, %rsi
;;       movq    %rsi, (%rdi)
;;       addq    $1, %rdx
;;       subq    $1, %rax
;;       jmp     0x1ca
;;  221: addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;  22a: ud2
;;  22c: ud2
;;  22e: ud2
;;  230: ud2
;;  232: ud2
