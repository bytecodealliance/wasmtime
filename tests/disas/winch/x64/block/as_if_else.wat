;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-if-else") (result i32)
      (if (result i32) (i32.const 1) (then (i32.const 2)) (else (block (result i32) (i32.const 1))))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4e
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       testl   %eax, %eax
;;       je      0x43
;;   39: movl    $2, %eax
;;       jmp     0x48
;;   43: movl    $1, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4e: ud2
