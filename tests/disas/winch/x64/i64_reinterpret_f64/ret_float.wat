;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        f64.const 1.0
        i64.reinterpret_f64
        drop
        f64.const 1.0
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x47
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x1c(%rip), %xmm0
;;       movq    %xmm0, %rax
;;       movsd   0xf(%rip), %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   47: ud2
;;   49: addb    %al, (%rax)
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %al, (%rax)
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %dh, %al
