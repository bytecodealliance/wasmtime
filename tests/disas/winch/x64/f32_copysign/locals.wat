;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f32)
        (local $foo f32)  
        (local $bar f32)

        (f32.const -1.1)
        (local.set $foo)

        (f32.const 2.2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        f32.copysign
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x7d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movss   0x44(%rip), %xmm0
;;   3c: movss   %xmm0, 4(%rsp)
;;   42: movss   0x3e(%rip), %xmm0
;;   4a: movss   %xmm0, (%rsp)
;;   4f: movss   (%rsp), %xmm0
;;   54: movss   4(%rsp), %xmm1
;;   5a: movl    $0x80000000, %r11d
;;   60: movd    %r11d, %xmm15
;;   65: andps   %xmm15, %xmm0
;;   69: andnps  %xmm1, %xmm15
;;   6d: movaps  %xmm15, %xmm1
;;   71: orps    %xmm0, %xmm1
;;   74: movaps  %xmm1, %xmm0
;;   77: addq    $0x18, %rsp
;;   7b: popq    %rbp
;;   7c: retq
;;   7d: ud2
;;   7f: addb    %cl, %ch
;;   81: int3
