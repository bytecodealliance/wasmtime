;;! target = "x86_64"
;;! test = "winch"
;;! flags = [ "-Ccranelift-has-avx" ]

(module
    (func (param i32) (result v128)
        (i16x8.replace_lane 1 (v128.const i64x2 1 2) (local.get 0))
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x20, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x48
;;   1c: movq    %rdi, %r14
;;       subq    $0x20, %rsp
;;       movq    %rdi, 0x18(%rsp)
;;       movq    %rsi, 0x10(%rsp)
;;       movl    %edx, 0xc(%rsp)
;;       movl    0xc(%rsp), %eax
;;       movdqu  0x13(%rip), %xmm0
;;       vpinsrw $1, %eax, %xmm0, %xmm0
;;       addq    $0x20, %rsp
;;       popq    %rbp
;;       retq
;;   48: ud2
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: addl    %eax, (%rax)
;;   52: addb    %al, (%rax)
;;   54: addb    %al, (%rax)
;;   56: addb    %al, (%rax)
;;   58: addb    (%rax), %al
;;   5a: addb    %al, (%rax)
;;   5c: addb    %al, (%rax)
;;   5e: addb    %al, (%rax)
