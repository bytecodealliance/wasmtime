;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x28, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x28, %rsp
;;   22: movq    %rdi, 0x20(%rsp)
;;   27: movq    %rsi, 0x18(%rsp)
;;   2c: movsd   %xmm0, 0x10(%rsp)
;;   32: movsd   %xmm1, 8(%rsp)
;;   38: movl    %edx, 4(%rsp)
;;   3c: movl    4(%rsp), %eax
;;   40: movsd   8(%rsp), %xmm0
;;   46: movsd   0x10(%rsp), %xmm1
;;   4c: cmpl    $0, %eax
;;   4f: je      0x59
;;   55: movsd   %xmm1, %xmm0
;;   59: addq    $0x28, %rsp
;;   5d: popq    %rbp
;;   5e: retq
;;   5f: ud2
