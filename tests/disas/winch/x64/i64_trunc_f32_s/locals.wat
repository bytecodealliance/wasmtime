;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f32)  

        (local.get 0)
        (i64.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x80
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    $0, 8(%rsp)
;;       movss   0xc(%rsp), %xmm0
;;       cvttss2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x77
;;   4a: ucomiss %xmm0, %xmm0
;;       jp      0x82
;;   53: movl    $0xdf000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm0
;;       jb      0x84
;;   68: xorpd   %xmm15, %xmm15
;;       ucomiss %xmm0, %xmm15
;;       jb      0x86
;;   77: addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   80: ud2
;;   82: ud2
;;   84: ud2
;;   86: ud2
