;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f32.const 1.0)
        (i64.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x78
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x49(%rip), %xmm0
;;       cvttss2si %xmm0, %rax
;;       cmpq    $1, %rax
;;       jno     0x6f
;;   42: ucomiss %xmm0, %xmm0
;;       jp      0x7a
;;   4b: movl    $0xdf000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm0
;;       jb      0x7c
;;   60: xorpd   %xmm15, %xmm15
;;       ucomiss %xmm0, %xmm15
;;       jb      0x7e
;;   6f: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   78: ud2
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2
;;   80: addb    %al, (%rax)
