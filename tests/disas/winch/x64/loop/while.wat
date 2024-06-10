;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "while-") (param i64) (result i64)
    (local i64)
    (local.set 1 (i64.const 1))
    (block
      (loop
        (br_if 1 (i64.eqz (local.get 0)))
        (local.set 1 (i64.mul (local.get 0) (local.get 1)))
        (local.set 0 (i64.sub (local.get 0) (i64.const 1)))
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
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x8c
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movq    $1, %rax
;;       movq    %rax, (%rsp)
;;       movq    8(%rsp), %rax
;;       cmpq    $0, %rax
;;       movl    $0, %eax
;;       sete    %al
;;       testl   %eax, %eax
;;       jne     0x82
;;   5e: movq    (%rsp), %rax
;;       movq    8(%rsp), %rcx
;;       imulq   %rax, %rcx
;;       movq    %rcx, (%rsp)
;;       movq    8(%rsp), %rax
;;       subq    $1, %rax
;;       movq    %rax, 8(%rsp)
;;       jmp     0x44
;;   82: movq    (%rsp), %rax
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   8c: ud2
