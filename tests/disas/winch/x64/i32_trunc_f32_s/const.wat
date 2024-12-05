;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f32.const 1.0)
        (i32.trunc_f32_s)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x74
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x4c(%rip), %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $1, %eax
;;       jno     0x6e
;;   41: ucomiss %xmm0, %xmm0
;;       jp      0x76
;;   4a: movl    $0xcf000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm0
;;       jb      0x78
;;   5f: xorpd   %xmm15, %xmm15
;;       ucomiss %xmm0, %xmm15
;;       jb      0x7a
;;   6e: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   74: ud2
;;   76: ud2
;;   78: ud2
;;   7a: ud2
;;   7c: addb    %al, (%rax)
;;   7e: addb    %al, (%rax)
;;   80: addb    %al, (%rax)
