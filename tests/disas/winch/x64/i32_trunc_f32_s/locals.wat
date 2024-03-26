;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f32)  

        (local.get 0)
        (i32.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x18, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x7a
;;   1b: movq    %rdi, %r14
;;       subq    $0x18, %rsp
;;       movq    %rdi, 0x10(%rsp)
;;       movq    %rsi, 8(%rsp)
;;       movq    $0, (%rsp)
;;       movss   4(%rsp), %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $1, %eax
;;       jno     0x74
;;   47: ucomiss %xmm0, %xmm0
;;       jp      0x7c
;;   50: movl    $0xcf000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm0
;;       jb      0x7e
;;   65: xorpd   %xmm15, %xmm15
;;       ucomiss %xmm0, %xmm15
;;       jb      0x80
;;   74: addq    $0x18, %rsp
;;       popq    %rbp
;;       retq
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
