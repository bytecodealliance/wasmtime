;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "select-i64") (param i64 i64 i32) (result i64)
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
;;   15: ja      0x59
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x28, %rsp
;;   22: movq    %rdi, 0x20(%rsp)
;;   27: movq    %rsi, 0x18(%rsp)
;;   2c: movq    %rdx, 0x10(%rsp)
;;   31: movq    %rcx, 8(%rsp)
;;   36: movl    %r8d, 4(%rsp)
;;   3b: movl    4(%rsp), %eax
;;   3f: movq    8(%rsp), %rcx
;;   44: movq    0x10(%rsp), %rdx
;;   49: cmpl    $0, %eax
;;   4c: cmovneq %rdx, %rcx
;;   50: movq    %rcx, %rax
;;   53: addq    $0x28, %rsp
;;   57: popq    %rbp
;;   58: retq
;;   59: ud2
