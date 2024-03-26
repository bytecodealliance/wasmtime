;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result i32)
        (local $foo f64)  
        (local $bar f64)

        (f64.const 1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.lt
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x71
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: xorl    %r11d, %r11d
;;   2f: movq    %r11, 8(%rsp)
;;   34: movq    %r11, (%rsp)
;;   38: movsd   0x38(%rip), %xmm0
;;   40: movsd   %xmm0, 8(%rsp)
;;   46: movsd   0x32(%rip), %xmm0
;;   4e: movsd   %xmm0, (%rsp)
;;   53: movsd   (%rsp), %xmm0
;;   58: movsd   8(%rsp), %xmm1
;;   5e: ucomisd %xmm1, %xmm0
;;   62: movl    $0, %eax
;;   67: seta    %al
;;   6b: addq    $0x20, %rsp
;;   6f: popq    %rbp
;;   70: retq
;;   71: ud2
;;   73: addb    %al, (%rax)
;;   75: addb    %al, (%rax)
;;   77: addb    %bl, -0x66666667(%rdx)
;;   7d: cltd
;;   7e: int1
