;;! target = "x86_64"
;;! test = "winch"

(module
    (func (result f64)
        (local i32)  

        (local.get 0)
        (f64.convert_i32_s)
    )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x42
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    4(%rsp), %eax
;;   38: cvtsi2sdl %eax, %xmm0
;;   3c: addq    $0x18, %rsp
;;   40: popq    %rbp
;;   41: retq
;;   42: ud2
