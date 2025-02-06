;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (i16x8.narrow_i32x4_u (v128.const i16x8 7 6 5 4 3 2 1 0) (v128.const i16x8 0 1 2 3 4 5 6 7))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4b
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       movdqu  0x24(%rip), %xmm1
;;       vpackusdw %xmm0, %xmm1, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4b: ud2
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rax)
;;   51: addb    %al, (%rcx)
;;   53: addb    %al, (%rdx)
;;   55: addb    %al, (%rbx)
;;   57: addb    %al, (%rax, %rax)
;;   5a: addl    $0x7000600, %eax
;;   5f: addb    %al, (%rdi)
;;   61: addb    %al, (%rsi)
;;   63: addb    %al, 0x3000400(%rip)
;;   69: addb    %al, (%rdx)
;;   6b: addb    %al, (%rcx)
;;   6d: addb    %al, (%rax)
