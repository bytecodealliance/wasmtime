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
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7c
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movss   4(%rsp), %xmm0
;;       cvttss2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x76
;;   49: ucomiss %xmm0, %xmm0
;;       jp      0x7e
;;   52: movl    $0xdf000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm0
;;       jb      0x80
;;   67: xorpd   %xmm15, %xmm15
;;       ucomiss %xmm0, %xmm15
;;       jb      0x82
;;   76: addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: ud2
