;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-i32") (param i32 i32 i32) (result i32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x53
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x20, %rsp
;;   22: movq    %rdi, 0x18(%rsp)
;;   27: movq    %rsi, 0x10(%rsp)
;;   2c: movl    %edx, 0xc(%rsp)
;;   30: movl    %ecx, 8(%rsp)
;;   34: movl    %r8d, 4(%rsp)
;;   39: movl    4(%rsp), %eax
;;   3d: movl    8(%rsp), %ecx
;;   41: movl    0xc(%rsp), %edx
;;   45: cmpl    $0, %eax
;;   48: cmovnel %edx, %ecx
;;   4b: movl    %ecx, %eax
;;   4d: addq    $0x20, %rsp
;;   51: popq    %rbp
;;   52: retq
;;   53: ud2
