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
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4f
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movl    %edx, 4(%rsp)
;;       movl    %ecx, (%rsp)
;;       movl    4(%rsp), %eax
;;       testl   %eax, %eax
;;       je      0x47
;;   3f: movl    (%rsp), %eax
;;       jmp     0x49
;;   47: ud2
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
