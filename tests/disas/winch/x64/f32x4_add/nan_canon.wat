;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Wnan-canonicalization", "-Ccranelift-has-avx"]

(module
    (func (param v128 v128) (result v128)
        local.get 0
        local.get 1
        f32x4.add
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x30, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x6c
;;   1c: movq    %rdi, %r14
;;       subq    $0x30, %rsp
;;       movq    %rdi, 0x28(%rsp)
;;       movq    %rsi, 0x20(%rsp)
;;       movdqu  %xmm0, 0x10(%rsp)
;;       movdqu  %xmm1, (%rsp)
;;       movdqu  (%rsp), %xmm0
;;       movdqu  0x10(%rsp), %xmm1
;;       vaddps  %xmm0, %xmm1, %xmm1
;;       vcmpunordps %xmm1, %xmm1, %xmm15
;;       vandnps %xmm1, %xmm15, %xmm1
;;       vandps  0x15(%rip), %xmm15, %xmm15
;;       vorps   %xmm1, %xmm15, %xmm1
;;       movdqa  %xmm1, %xmm0
;;       addq    $0x30, %rsp
;;       popq    %rbp
;;       retq
;;   6c: ud2
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rax)
;;   72: sarb    $0, (%rdi)
;;   76: sarb    $0, (%rdi)
;;   7a: sarb    $0, (%rdi)
