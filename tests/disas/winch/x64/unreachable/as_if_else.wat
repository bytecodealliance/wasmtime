;;! target = "x86_64"
;;! test = "winch"


(module
  (func (export "as-if-else") (param i32 i32) (result i32)
    (if (result i32) (local.get 0) (then (local.get 1)) (else (unreachable)))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x51
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    %ecx, 8(%rsp)
;;       movl    0xc(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x49
;;   40: movl    8(%rsp), %eax
;;       jmp     0x4b
;;   49: ud2
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   51: ud2
