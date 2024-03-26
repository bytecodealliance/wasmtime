;;! target = "x86_64"
;;! test = "winch"

(module
  (func $dummy3 (param i32 i32 i32))
  (func (export "as-call-first")
    (call $dummy3 (unreachable) (i32.const 2) (i32.const 3))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x3f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movl    %edx, 0xc(%rsp)
;;   30: movl    %ecx, 8(%rsp)
;;   34: movl    %r8d, 4(%rsp)
;;   39: addq    $0x20, %rsp
;;   3d: popq    %rbp
;;   3e: retq
;;   3f: ud2
;;
;; wasm[0]::function[1]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: movq    8(%rdi), %r11
;;   58: movq    (%r11), %r11
;;   5b: addq    $0x10, %r11
;;   62: cmpq    %rsp, %r11
;;   65: ja      0x83
;;   6b: movq    %rdi, %r14
;;   6e: subq    $0x10, %rsp
;;   72: movq    %rdi, 8(%rsp)
;;   77: movq    %rsi, (%rsp)
;;   7b: ud2
;;   7d: addq    $0x10, %rsp
;;   81: popq    %rbp
;;   82: retq
;;   83: ud2
