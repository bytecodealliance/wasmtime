;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (result v128)
        (f64x2.convert_low_i32x4_u (v128.const i64x2 1 0))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movdqu  0x1c(%rip), %xmm0
;;       vunpcklps 0x24(%rip), %xmm0, %xmm0
;;       vsubpd  0x2c(%rip), %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4a: ud2
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: addl    %eax, (%rax)
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: addb    %al, (%rax)
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
;;   60: addb    %al, (%rax)
;;   62: xorb    %al, (%rbx)
;;   65: addb    %dh, (%rax)
;;   67: addb    %al, (%r8)
;;   6a: addb    %al, (%rax)
;;   6c: addb    %al, (%rax)
;;   6e: addb    %al, (%rax)
;;   70: addb    %al, (%rax)
;;   72: addb    %al, (%rax)
;;   74: addb    %al, (%rax)
;;   76: xorb    %al, (%rbx)
;;   79: addb    %al, (%rax)
;;   7b: addb    %al, (%rax)
;;   7d: addb    %dh, (%rax)
