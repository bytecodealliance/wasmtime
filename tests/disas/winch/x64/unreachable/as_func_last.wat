;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "as-func-last")
    (call $dummy) (unreachable)
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x31
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: addq    $0x10, %rsp
;;   2f: popq    %rbp
;;   30: retq
;;   31: ud2
;;
;; wasm[0]::function[1]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: movq    8(%rdi), %r11
;;   48: movq    (%r11), %r11
;;   4b: addq    $0x10, %r11
;;   52: cmpq    %rsp, %r11
;;   55: ja      0x83
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x10, %rsp
;;   62: movq    %rdi, 8(%rsp)
;;   67: movq    %rsi, (%rsp)
;;   6b: movq    %r14, %rdi
;;   6e: movq    %r14, %rsi
;;   71: callq   0
;;   76: movq    8(%rsp), %r14
;;   7b: ud2
;;   7d: addq    $0x10, %rsp
;;   81: popq    %rbp
;;   82: retq
;;   83: ud2
