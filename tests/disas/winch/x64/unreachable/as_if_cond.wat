;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "as-if-cond") (result i32)
    (if (result i32) (unreachable) (then (i32.const 0)) (else (i32.const 1)))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x33
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: ud2
;;   2d: addq    $0x10, %rsp
;;   31: popq    %rbp
;;   32: retq
;;   33: ud2
