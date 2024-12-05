;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.32)
        (f64.neg)
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
;;       movsd   0x1c(%rip), %xmm0
;;       movabsq $9223372036854775808, %r11
;;       movq    %r11, %xmm15
;;       xorpd   %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4e: ud2
