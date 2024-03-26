;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i64)
        (f64.const 1.0)
        (i64.trunc_f64_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7c
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x55(%rip), %xmm0
;;   33: cvttsd2si %xmm0, %rax
;;   38: cmpq    $1, %rax
;;   3c: jno     0x76
;;   42: ucomisd %xmm0, %xmm0
;;   46: jp      0x7e
;;   4c: movabsq $14114281232179134464, %r11
;;   56: movq    %r11, %xmm15
;;   5b: ucomisd %xmm15, %xmm0
;;   60: jb      0x80
;;   66: xorpd   %xmm15, %xmm15
;;   6b: ucomisd %xmm0, %xmm15
;;   70: jb      0x82
;;   76: addq    $0x10, %rsp
;;   7a: popq    %rbp
;;   7b: retq
;;   7c: ud2
;;   7e: ud2
;;   80: ud2
;;   82: ud2
;;   84: addb    %al, (%rax)
;;   86: addb    %al, (%rax)
;;   88: addb    %al, (%rax)
;;   8a: addb    %al, (%rax)
;;   8c: addb    %al, (%rax)
