;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "type-i32") (result i32) (unreachable))
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x33
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       ud2
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   33: ud2
