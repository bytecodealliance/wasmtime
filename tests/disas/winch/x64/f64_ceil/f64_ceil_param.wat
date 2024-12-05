;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result f64)
        (local.get 0)
        (f64.ceil)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6e
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   8(%rsp), %xmm15
;;       subq    $8, %rsp
;;       movsd   %xmm15, (%rsp)
;;       subq    $8, %rsp
;;       movsd   8(%rsp), %xmm0
;;       movabsq $0, %r11
;;       callq   *%r11
;;       addq    $8, %rsp
;;       addq    $8, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   6e: ud2
