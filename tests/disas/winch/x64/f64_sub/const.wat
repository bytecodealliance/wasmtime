;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.sub)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x49
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x1d(%rip), %xmm0
;;   33: movsd   0x1d(%rip), %xmm1
;;   3b: subsd   %xmm0, %xmm1
;;   3f: movapd  %xmm1, %xmm0
;;   43: addq    $0x10, %rsp
;;   47: popq    %rbp
;;   48: retq
;;   49: ud2
;;   4b: addb    %al, (%rax)
;;   4d: addb    %al, (%rax)
;;   4f: addb    %bl, -0x66666667(%rdx)
;;   55: cltd
;;   56: addl    %eax, -0x66(%rax)
;;   59: cltd
;;   5a: cltd
;;   5b: cltd
;;   5c: cltd
;;   5d: cltd
;;   5e: int1
