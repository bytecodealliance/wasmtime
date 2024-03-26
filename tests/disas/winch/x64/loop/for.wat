;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "for-") (param i64) (result i64)
    (local i64 i64)
    (local.set 1 (i64.const 1))
    (local.set 2 (i64.const 2))
    (block
      (loop
        (br_if 1 (i64.gt_u (local.get 2) (local.get 0)))
        (local.set 1 (i64.mul (local.get 1) (local.get 2)))
        (local.set 2 (i64.add (local.get 2) (i64.const 1)))
        (br 0)
      )
    )
    (local.get 1)
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x28, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x9f
;;   1b: movq    %rdi, %r14
;;       subq    $0x28, %rsp
;;       movq    %rdi, 0x20(%rsp)
;;       movq    %rsi, 0x18(%rsp)
;;       movq    %rdx, 0x10(%rsp)
;;       xorl    %r11d, %r11d
;;       movq    %r11, 8(%rsp)
;;       movq    %r11, (%rsp)
;;       movq    $1, %rax
;;       movq    %rax, 8(%rsp)
;;       movq    $2, %rax
;;       movq    %rax, (%rsp)
;;       movq    0x10(%rsp), %rax
;;       movq    (%rsp), %rcx
;;       cmpq    %rax, %rcx
;;       movl    $0, %ecx
;;       seta    %cl
;;       testl   %ecx, %ecx
;;       jne     0x94
;;   71: movq    (%rsp), %rax
;;       movq    8(%rsp), %rcx
;;       imulq   %rax, %rcx
;;       movq    %rcx, 8(%rsp)
;;       movq    (%rsp), %rax
;;       addq    $1, %rax
;;       movq    %rax, (%rsp)
;;       jmp     0x54
;;   94: movq    8(%rsp), %rax
;;       addq    $0x28, %rsp
;;       popq    %rbp
;;       retq
;;   9f: ud2
