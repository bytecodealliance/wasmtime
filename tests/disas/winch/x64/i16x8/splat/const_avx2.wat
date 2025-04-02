;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx2" ]

(module
    (func (result v128)
        (i16x8.splat (i32.const 0))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       vpbroadcastw 0xb(%rip), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3b: ud2
;;   3d: addb    %al, (%rax)
;;   3f: addb    %al, (%rax)
;;   41: addb    %al, (%rax)
