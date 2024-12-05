;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.neg)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x49
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x1c(%rip), %xmm0
;;       movl    $0x80000000, %r11d
;;       movd    %r11d, %xmm15
;;       xorps   %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   49: ud2
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, %bl
;;   51: cmc
;;   52: testb   $0xbf, %al
