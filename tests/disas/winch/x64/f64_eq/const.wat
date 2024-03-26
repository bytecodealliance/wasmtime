;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (f64.const 1.1)
        (f64.const 2.2)
        (f64.eq)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x2d(%rip), %xmm0
;;   33: movsd   0x2d(%rip), %xmm1
;;   3b: ucomisd %xmm0, %xmm1
;;   3f: movl    $0, %eax
;;   44: sete    %al
;;   48: movl    $0, %r11d
;;   4e: setnp   %r11b
;;   52: andq    %r11, %rax
;;   55: addq    $0x10, %rsp
;;   59: popq    %rbp
;;   5a: retq
;;   5b: ud2
;;   5d: addb    %al, (%rax)
;;   5f: addb    %bl, -0x66666667(%rdx)
;;   65: cltd
;;   66: addl    %eax, -0x66(%rax)
;;   69: cltd
;;   6a: cltd
;;   6b: cltd
;;   6c: cltd
;;   6d: cltd
;;   6e: int1
