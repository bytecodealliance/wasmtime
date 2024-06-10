;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.abs)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x50
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movsd   %xmm0, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       movabsq $0x7fffffffffffffff, %r11
;;       movq    %r11, %xmm15
;;       andpd   %xmm15, %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   50: ud2
