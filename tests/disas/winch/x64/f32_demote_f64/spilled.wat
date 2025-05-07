;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        f64.const 1.0
        f32.demote_f64
        block
        end
    )
)
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x10(%r11), %r11
;;       addq    $0x14, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x5c
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       movsd   0x29(%rip), %xmm0
;;       cvtsd2ss %xmm0, %xmm0
;;       subq    $4, %rsp
;;       movss   %xmm0, (%rsp)
;;       movss   (%rsp), %xmm0
;;       addq    $4, %rsp
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   5c: ud2
;;   5e: addb    %al, (%rax)
;;   60: addb    %al, (%rax)
;;   62: addb    %al, (%rax)
;;   64: addb    %al, (%rax)
