;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "as-test-operand") (param i32) (result i32)
    (i32.eqz
      (if (result i32) (local.get 0)
        (then (call $dummy) (i32.const 13))
        (else (call $dummy) (i32.const 0))
      )
    )
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
;;       ja      0xcd
;;   5b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    4(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x9e
;;   7c: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $0xd, %eax
;;       jmp     0xbb
;;   9e: subq    $8, %rsp
;;       movq    %r14, %rdi
;;       movq    %r14, %rsi
;;       callq   0
;;       addq    $8, %rsp
;;       movq    0x10(%rsp), %r14
;;       movl    $0, %eax
;;       cmpl    $0, %eax
;;       movl    $0, %eax
;;       sete    %al
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   cd: ud2
