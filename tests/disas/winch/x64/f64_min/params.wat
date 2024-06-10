;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (param f64) (result f64)
        (local.get 0)
        (local.get 1)
        (f64.min)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x73
;;   1b: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movsd   %xmm0, 8(%rsp)
;;       movsd   %xmm1, (%rsp)
;;       movsd   (%rsp), %xmm0
;;       movsd   8(%rsp), %xmm1
;;       ucomisd %xmm0, %xmm1
;;       jne     0x65
;;       jp      0x5b
;;   52: orpd    %xmm0, %xmm1
;;       jmp     0x69
;;   5b: addsd   %xmm0, %xmm1
;;       jp      0x69
;;   65: minsd   %xmm0, %xmm1
;;       movapd  %xmm1, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   73: ud2
