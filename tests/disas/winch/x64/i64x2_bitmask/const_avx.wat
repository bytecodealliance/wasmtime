;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result i32)
        (i64x2.bitmask (v128.const i64x2 0 1))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x44
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x19(%rip), %xmm0
;;       vmovmskpd %xmm0, %eax
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   44: ud2
;;   46: addb    %al, (%rax)
;;   48: addb    %al, (%rax)
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: addb    %al, (%rax)
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: addl    %eax, (%rax)
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
