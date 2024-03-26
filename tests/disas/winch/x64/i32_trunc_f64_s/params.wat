;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i32)
        (local.get 0)
        (i32.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movsd   %xmm0, (%rsp)
;;   31: movsd   (%rsp), %xmm0
;;   36: cvttsd2si %xmm0, %eax
;;   3a: cmpl    $1, %eax
;;   3d: jno     0x77
;;   43: ucomisd %xmm0, %xmm0
;;   47: jp      0x7f
;;   4d: movabsq $13970166044105375744, %r11
;;   57: movq    %r11, %xmm15
;;   5c: ucomisd %xmm15, %xmm0
;;   61: jbe     0x81
;;   67: xorpd   %xmm15, %xmm15
;;   6c: ucomisd %xmm0, %xmm15
;;   71: jb      0x83
;;   77: addq    $0x18, %rsp
;;   7b: popq    %rbp
;;   7c: retq
;;   7d: ud2
;;   7f: ud2
;;   81: ud2
;;   83: ud2
