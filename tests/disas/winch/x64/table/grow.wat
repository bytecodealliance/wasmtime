;;! target = "x86_64"
;;! test = "winch"

(module
  (table $t1 0 funcref)

  (func (export "grow-by-10") (param $r funcref) (result i32)
    (table.grow $t1 (local.get $r) (i32.const 10))
  )
)


;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    %rdx, (%rsp)
;;   30: movq    (%rsp), %r11
;;   34: pushq   %r11
;;   36: movq    %r14, %rdi
;;   39: movl    $0, %esi
;;   3e: movl    $0xa, %edx
;;   43: movq    (%rsp), %rcx
;;   47: callq   0x18c
;;   4c: addq    $8, %rsp
;;   50: movq    0x10(%rsp), %r14
;;   55: addq    $0x18, %rsp
;;   59: popq    %rbp
;;   5a: retq
;;   5b: ud2
