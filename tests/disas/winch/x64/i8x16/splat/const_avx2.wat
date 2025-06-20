;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx2" ]

(module
    (func (result v128)
        (i8x16.splat (i32.const 0))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x41
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       vpbroadcastb 0x10(%rip), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   41: ud2
;;   43: addb    %al, (%rax)
;;   45: addb    %al, (%rax)
;;   47: addb    %al, (%rax)
;;   49: addb    %al, (%rax)
