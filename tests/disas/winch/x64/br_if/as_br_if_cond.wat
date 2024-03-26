;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-br-if-cond")
    (block (br_if 0 (br_if 0 (i32.const 1) (i32.const 1))))
  )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4b
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movl    $1, %eax
;;       testl   %eax, %eax
;;       jne     0x45
;;   38: movl    $1, %eax
;;       testl   %eax, %eax
;;       jne     0x45
;;   45: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4b: ud2
