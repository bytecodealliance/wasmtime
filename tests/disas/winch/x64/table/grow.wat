;;! target = "x86_64"
;;! test = "winch"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
)


;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x108
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movl    $0xa, %eax
;;       movl    %eax, %ecx
;;       movq    8(%rsp), %r11
;;       pushq   %r11
;;       subq    $4, %rsp
;;       movl    %ecx, (%rsp)
;;       subq    $4, %rsp
;;       movl    %eax, (%rsp)
;;       movq    %r14, %rdi
;;       movl    $0, %esi
;;       movl    (%rsp), %edx
;;       callq   0x20f
;;       addq    $4, %rsp
;;       movq    0x24(%rsp), %r14
;;       movl    (%rsp), %ecx
;;       addq    $4, %rsp
;;       popq    %rdx
;;       movq    %rax, %rbx
;;       cmpq    $-1, %rax
;;       je      0xfb
;;   8b: movq    %r14, %r11
;;       movq    0x38(%r11), %rsi
;;       movl    %eax, %edi
;;       addl    %ecx, %edi
;;       jb      0x10a
;;   9c: cmpl    %esi, %edi
;;       ja      0x10c
;;   a4: cmpq    $0, %rcx
;;       je      0xfb
;;   ae: movq    %rax, %rsi
;;       movq    %rdx, %rdi
;;       movq    %r14, %r8
;;       movq    0x38(%r8), %r9
;;       cmpq    %r9, %rsi
;;       jae     0x10e
;;   c4: movq    %rsi, %r11
;;       imulq   $8, %r11, %r11
;;       movq    0x30(%r8), %r8
;;       movq    %r8, %r10
;;       addq    %r11, %r8
;;       cmpq    %r9, %rsi
;;       cmovaeq %r10, %r8
;;       orq     $1, %rdi
;;       movq    %rdi, (%r8)
;;       addq    $1, %rax
;;       subq    $1, %rcx
;;       jmp     0xa4
;;   fb: movl    %ebx, %ebx
;;       movl    %ebx, %eax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;  108: ud2
;;  10a: ud2
;;  10c: ud2
;;  10e: ud2
