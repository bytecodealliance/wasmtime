;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f32.const -1.32)
        (f32.abs)
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x48
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x1d(%rip), %xmm0
;;       movl    $0x7fffffff, %r11d
;;       movd    %r11d, %xmm15
;;       andps   %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   48: ud2
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
;;   4e: addb    %al, (%rax)
;;   50: retq
;;   51: cmc
;;   52: testb   $0xbf, %al
