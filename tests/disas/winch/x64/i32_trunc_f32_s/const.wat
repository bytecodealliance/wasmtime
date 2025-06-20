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
;;       ja      0x7a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x51(%rip), %xmm0
;;       cvttss2si %xmm0, %eax
;;       cmpl    $1, %eax
;;       jno     0x71
;;   44: ucomiss %xmm0, %xmm0
;;       jp      0x7c
;;   4d: movl    $0xcf000000, %r11d
;;       movd    %r11d, %xmm15
;;       ucomiss %xmm15, %xmm0
;;       jb      0x7e
;;   62: xorpd   %xmm15, %xmm15
;;       ucomiss %xmm0, %xmm15
;;       jb      0x80
;;   71: addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: addb    %al, (%rax)
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)
