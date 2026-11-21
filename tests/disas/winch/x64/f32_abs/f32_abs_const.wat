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
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x4f
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movss   0x21(%rip), %xmm0
;;       movl    $0x7fffffff, %r11d
;;       movd    %r11d, %xmm15
;;       andps   %xmm15, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   4f: ud2
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, %bl
;;   59: cmc
;;   5a: testb   $0xbf, %al
