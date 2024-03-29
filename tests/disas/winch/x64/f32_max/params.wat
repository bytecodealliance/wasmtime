;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f32) (param f32) (result f32)
        (local.get 0)
        (local.get 1)
        (f32.max)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x70
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movss   %xmm0, 4(%rsp)
;;       movss   %xmm1, (%rsp)
;;       movss   (%rsp), %xmm0
;;       movss   4(%rsp), %xmm1
;;       ucomiss %xmm0, %xmm1
;;       jne     0x63
;;       jp      0x59
;;   51: andps   %xmm0, %xmm1
;;       jmp     0x67
;;   59: addss   %xmm0, %xmm1
;;       jp      0x67
;;   63: maxss   %xmm0, %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   70: ud2
