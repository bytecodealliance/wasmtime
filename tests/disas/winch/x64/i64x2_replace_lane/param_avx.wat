;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (param i64) (result v128)
        (i64x2.replace_lane 1 (v128.const i64x2 1 2) (local.get 0))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4b
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movq    %rdx, 8(%rsp)
;;       movq    8(%rsp), %rax
;;       movdqu  0x11(%rip), %xmm0
;;       vpinsrq $1, %rax, %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   4b: ud2
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rcx)
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rdx)
;;   59: addb    %al, (%rax)
;;   5b: addb    %al, (%rax)
;;   5d: addb    %al, (%rax)
