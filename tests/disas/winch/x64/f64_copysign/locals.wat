;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (local $foo f64)  
        (local $bar f64)

        (f64.const -1.1)
        (local.set $foo)

        (f64.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f64.copysign
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x8a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: xorl    %r11d, %r11d
;;   2f: movq    %r11, 8(%rsp)
;;   34: movq    %r11, (%rsp)
;;   38: movsd   0x50(%rip), %xmm0
;;   40: movsd   %xmm0, 8(%rsp)
;;   46: movsd   0x4a(%rip), %xmm0
;;   4e: movsd   %xmm0, (%rsp)
;;   53: movsd   (%rsp), %xmm0
;;   58: movsd   8(%rsp), %xmm1
;;   5e: movabsq $9223372036854775808, %r11
;;   68: movq    %r11, %xmm15
;;   6d: andpd   %xmm15, %xmm0
;;   72: andnpd  %xmm1, %xmm15
;;   77: movapd  %xmm15, %xmm1
;;   7c: orpd    %xmm0, %xmm1
;;   80: movapd  %xmm1, %xmm0
;;   84: addq    $0x20, %rsp
;;   88: popq    %rbp
;;   89: retq
;;   8a: ud2
;;   8c: addb    %al, (%rax)
;;   8e: addb    %al, (%rax)
