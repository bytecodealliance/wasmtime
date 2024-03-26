;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "as-unary-operand") (result i32)
    (i32.ctz (loop (result i32) (call $dummy) (i32.const 13)))
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
;;   55: ja      0x9a
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x10, %rsp
;;   62: movq    %rdi, 8(%rsp)
;;   67: movq    %rsi, (%rsp)
;;   6b: movq    %r14, %rdi
;;   6e: movq    %r14, %rsi
;;   71: callq   0
;;   76: movq    8(%rsp), %r14
;;   7b: movl    $0xd, %eax
;;   80: bsfl    %eax, %eax
;;   83: movl    $0, %r11d
;;   89: sete    %r11b
;;   8d: shll    $5, %r11d
;;   91: addl    %r11d, %eax
;;   94: addq    $0x10, %rsp
;;   98: popq    %rbp
;;   99: retq
;;   9a: ud2
