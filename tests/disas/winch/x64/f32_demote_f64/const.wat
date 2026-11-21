;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (f64.const 1.0)
        (f32.demote_f64)
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
;;       movsd   0x11(%rip), %xmm0
;;       cvtsd2ss %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   44: ud2
;;   46: addb    %al, (%rax)
;;   48: addb    %al, (%rax)
;;   4a: addb    %al, (%rax)
;;   4c: addb    %al, (%rax)
