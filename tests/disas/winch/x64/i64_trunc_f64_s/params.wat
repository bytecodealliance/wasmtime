;;! target = "x86_64"
;;! test = "winch"

(module
    (func (param f64) (result i64)
        (local.get 0)
        (i64.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movsd   %xmm0, (%rsp)
;;   31: movsd   (%rsp), %xmm0
;;   36: cvttsd2si %xmm0, %rax
;;   3b: cmpq    $1, %rax
;;   3f: jno     0x79
;;   45: ucomisd %xmm0, %xmm0
;;   49: jp      0x81
;;   4f: movabsq $14114281232179134464, %r11
;;   59: movq    %r11, %xmm15
;;   5e: ucomisd %xmm15, %xmm0
;;   63: jb      0x83
;;   69: xorpd   %xmm15, %xmm15
;;   6e: ucomisd %xmm0, %xmm15
;;   73: jb      0x85
;;   79: addq    $0x18, %rsp
;;   7d: popq    %rbp
;;   7e: retq
;;   7f: ud2
;;   81: ud2
;;   83: ud2
;;   85: ud2
