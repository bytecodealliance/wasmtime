;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (local f64)  

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
;;   15: ja      0x82
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movsd   (%rsp), %xmm0
;;   39: cvttsd2si %xmm0, %rax
;;   3e: cmpq    $1, %rax
;;   42: jno     0x7c
;;   48: ucomisd %xmm0, %xmm0
;;   4c: jp      0x84
;;   52: movabsq $14114281232179134464, %r11
;;   5c: movq    %r11, %xmm15
;;   61: ucomisd %xmm15, %xmm0
;;   66: jb      0x86
;;   6c: xorpd   %xmm15, %xmm15
;;   71: ucomisd %xmm0, %xmm15
;;   76: jb      0x88
;;   7c: addq    $0x18, %rsp
;;   80: popq    %rbp
;;   81: retq
;;   82: ud2
;;   84: ud2
;;   86: ud2
;;   88: ud2
