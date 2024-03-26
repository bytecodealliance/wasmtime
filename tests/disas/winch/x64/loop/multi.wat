;;! target = "x86_64"
;;! test = "winch"

(module
  (func $dummy)
  (func (export "multi") (result i32)
    (loop (call $dummy) (call $dummy) (call $dummy) (call $dummy))
    (loop (result i32) (call $dummy) (call $dummy) (i32.const 8) (call $dummy))
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
;;   55: ja      0xe6
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x10, %rsp
;;   62: movq    %rdi, 8(%rsp)
;;   67: movq    %rsi, (%rsp)
;;   6b: movq    %r14, %rdi
;;   6e: movq    %r14, %rsi
;;   71: callq   0
;;   76: movq    8(%rsp), %r14
;;   7b: movq    %r14, %rdi
;;   7e: movq    %r14, %rsi
;;   81: callq   0
;;   86: movq    8(%rsp), %r14
;;   8b: movq    %r14, %rdi
;;   8e: movq    %r14, %rsi
;;   91: callq   0
;;   96: movq    8(%rsp), %r14
;;   9b: movq    %r14, %rdi
;;   9e: movq    %r14, %rsi
;;   a1: callq   0
;;   a6: movq    8(%rsp), %r14
;;   ab: movq    %r14, %rdi
;;   ae: movq    %r14, %rsi
;;   b1: callq   0
;;   b6: movq    8(%rsp), %r14
;;   bb: movq    %r14, %rdi
;;   be: movq    %r14, %rsi
;;   c1: callq   0
;;   c6: movq    8(%rsp), %r14
;;   cb: movq    %r14, %rdi
;;   ce: movq    %r14, %rsi
;;   d1: callq   0
;;   d6: movq    8(%rsp), %r14
;;   db: movl    $8, %eax
;;   e0: addq    $0x10, %rsp
;;   e4: popq    %rbp
;;   e5: retq
;;   e6: ud2
