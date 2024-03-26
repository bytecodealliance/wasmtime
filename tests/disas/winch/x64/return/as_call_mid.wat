;;! target = "x86_64"
;;! test = "winch"
(module
  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-mid") (result i32)
    (call $f (i32.const 1) (return (i32.const 13)) (i32.const 3))
  )
)
  
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x44
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movl    %edx, 0xc(%rsp)
;;   30: movl    %ecx, 8(%rsp)
;;   34: movl    %r8d, 4(%rsp)
;;   39: movl    $0xffffffff, %eax
;;   3e: addq    $0x20, %rsp
;;   42: popq    %rbp
;;   43: retq
;;   44: ud2
;;
;; wasm[0]::function[1]:
;;   50: pushq   %rbp
;;   51: movq    %rsp, %rbp
;;   54: movq    8(%rdi), %r11
;;   58: movq    (%r11), %r11
;;   5b: addq    $0x10, %r11
;;   62: cmpq    %rsp, %r11
;;   65: ja      0x86
;;   6b: movq    %rdi, %r14
;;   6e: subq    $0x10, %rsp
;;   72: movq    %rdi, 8(%rsp)
;;   77: movq    %rsi, (%rsp)
;;   7b: movl    $0xd, %eax
;;   80: addq    $0x10, %rsp
;;   84: popq    %rbp
;;   85: retq
;;   86: ud2
