;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (f64.const -1.1)
        (f64.const 2.2)
        (f64.copysign)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x67
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movsd   0x3d(%rip), %xmm0
;;   33: movsd   0x3d(%rip), %xmm1
;;   3b: movabsq $9223372036854775808, %r11
;;   45: movq    %r11, %xmm15
;;   4a: andpd   %xmm15, %xmm0
;;   4f: andnpd  %xmm1, %xmm15
;;   54: movapd  %xmm15, %xmm1
;;   59: orpd    %xmm0, %xmm1
;;   5d: movapd  %xmm1, %xmm0
;;   61: addq    $0x10, %rsp
;;   65: popq    %rbp
;;   66: retq
;;   67: ud2
;;   69: addb    %al, (%rax)
;;   6b: addb    %al, (%rax)
;;   6d: addb    %al, (%rax)
;;   6f: addb    %bl, -0x66666667(%rdx)
;;   75: cltd
;;   76: addl    %eax, -0x66(%rax)
;;   79: cltd
;;   7a: cltd
;;   7b: cltd
;;   7c: cltd
;;   7d: cltd
;;   7e: int1
