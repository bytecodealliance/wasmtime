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
;;       movq    (%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3d
;;   1b: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0xd(%rip), %xmm0
;;       cvtsd2ss %xmm0, %xmm0
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3d: ud2
;;   3f: addb    %al, (%rax)
;;   41: addb    %al, (%rax)
;;   43: addb    %al, (%rax)
;;   45: addb    %dh, %al
