;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.0)
        (i32.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x55(%rip), %xmm0
;;   33: cvttsd2si %xmm0, %eax
;;   37: cmpl    $1, %eax
;;   3a: jno     0x74
;;   40: ucomisd %xmm0, %xmm0
;;   44: jp      0x7c
;;   4a: movabsq $13970166044105375744, %r11
;;   54: movq    %r11, %xmm15
;;   59: ucomisd %xmm15, %xmm0
;;   5e: jbe     0x7e
;;   64: xorpd   %xmm15, %xmm15
;;   69: ucomisd %xmm0, %xmm15
;;   6e: jb      0x80
;;   74: addq    $0x10, %rsp
;;   78: popq    %rbp
;;   79: retq
;;   7a: ud2
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: addb    %al, (%rax)
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
