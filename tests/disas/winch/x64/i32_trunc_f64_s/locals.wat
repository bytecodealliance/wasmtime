;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local f64)  

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
;;   15: ja      0x80
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movsd   (%rsp), %xmm0
;;   39: cvttsd2si %xmm0, %eax
;;   3d: cmpl    $1, %eax
;;   40: jno     0x7a
;;   46: ucomisd %xmm0, %xmm0
;;   4a: jp      0x82
;;   50: movabsq $13970166044105375744, %r11
;;   5a: movq    %r11, %xmm15
;;   5f: ucomisd %xmm15, %xmm0
;;   64: jbe     0x84
;;   6a: xorpd   %xmm15, %xmm15
;;   6f: ucomisd %xmm0, %xmm15
;;   74: jb      0x86
;;   7a: addq    $0x18, %rsp
;;   7e: popq    %rbp
;;   7f: retq
;;   80: ud2
;;   82: ud2
;;   84: ud2
;;   86: ud2
