;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        f32.const 1.0
        f64.promote_f32
        block
        end
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movss   0x25(%rip), %xmm0
;;   33: cvtss2sd %xmm0, %xmm0
;;   37: subq    $8, %rsp
;;   3b: movsd   %xmm0, (%rsp)
;;   40: movsd   (%rsp), %xmm0
;;   45: addq    $8, %rsp
;;   49: addq    $0x10, %rsp
;;   4d: popq    %rbp
;;   4e: retq
;;   4f: ud2
;;   51: addb    %al, (%rax)
;;   53: addb    %al, (%rax)
;;   55: addb    %al, (%rax)
;;   57: addb    %al, (%rax)
