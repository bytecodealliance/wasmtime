;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-Ccranelift-has_sse41"]

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.ceil)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x40
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x14(%rip), %xmm0
;;       roundss $2, %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   40: ud2
;;   42: addb    %al, (%rax)
;;   44: addb    %al, (%rax)
;;   46: addb    %al, (%rax)
;;   48: retq
;;   49: cmc
;;   4a: testb   $0xbf, %al
