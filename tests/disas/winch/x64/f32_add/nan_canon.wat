;;! target = "x86_64"
;;! test = "winch"
;;! flags = "-Wnan-canonicalization"

(module
    (func (param f32 f32) (result f32)
        local.get 0
        local.get 1
        f32.add
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x69
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movss   %xmm0, 0xc(%rsp)
;;       movss   %xmm1, 8(%rsp)
;;       movss   8(%rsp), %xmm0
;;       movss   0xc(%rsp), %xmm1
;;       addss   %xmm0, %xmm1
;;       ucomiss %xmm1, %xmm1
;;       jnp     0x5d
;;   55: movss   0x13(%rip), %xmm1
;;       movaps  %xmm1, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   69: ud2
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %al, (%rax)
;;   71: addb    %al, %al
