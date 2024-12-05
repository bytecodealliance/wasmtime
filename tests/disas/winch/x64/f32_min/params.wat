;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.min)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x73
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   %xmm1, 8(%rsp)
;;       movss   8(%rsp), %xmm0
;;       movss   0xc(%rsp), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x66
;;       jp      0x5c
;;   54: orps    %xmm0, %xmm1
;;       jmp     0x6a
;;   5c: addss   %xmm0, %xmm1
;;       jp      0x6a
;;   66: minss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   73: ud2
