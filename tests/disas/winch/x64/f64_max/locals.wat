;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.max
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x8f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: xorl    %r11d, %r11d
;;   2f: movq    %r11, 8(%rsp)
;;   34: movq    %r11, (%rsp)
;;   38: movsd   0x58(%rip), %xmm0
;;   40: movsd   %xmm0, 8(%rsp)
;;   46: movsd   0x52(%rip), %xmm0
;;   4e: movsd   %xmm0, (%rsp)
;;   53: movsd   (%rsp), %xmm0
;;   58: movsd   8(%rsp), %xmm1
;;   5e: ucomisd %xmm0, %xmm1
;;   62: jne     0x81
;;   68: jp      0x77
;;   6e: andpd   %xmm0, %xmm1
;;   72: jmp     0x85
;;   77: addsd   %xmm0, %xmm1
;;   7b: jp      0x85
;;   81: maxsd   %xmm0, %xmm1
;;   85: movapd  %xmm1, %xmm0
;;   89: addq    $0x20, %rsp
;;   8d: popq    %rbp
;;   8e: retq
;;   8f: ud2
;;   91: addb    %al, (%rax)
;;   93: addb    %al, (%rax)
;;   95: addb    %al, (%rax)
;;   97: addb    %bl, -0x66666667(%rdx)
;;   9d: cltd
;;   9e: int1
