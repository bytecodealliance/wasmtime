;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (result f32)
        (local.get 0)
        (f32.nearest)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6d
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   0xc(%rsp), %xmm15
;;       subq    $4, %rsp
;;       movss   %xmm15, (%rsp)
;;       subq    $0xc, %rsp
;;       movss   0xc(%rsp), %xmm0
;;       movabsq $0, %r11
;;       callq   *%r11
;;       addq    $0xc, %rsp
;;       addq    $4, %rsp
;;       movq    0x18(%rsp), %r14
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   6d: ud2
