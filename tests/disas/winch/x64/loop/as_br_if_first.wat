;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-br-if-first") (result i32)
    (block (result i32) (br_if 0 (loop (result i32) (i32.const 1)) (i32.const 2)))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x43
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $2, %ecx
;;   30: movl    $1, %eax
;;   35: testl   %ecx, %ecx
;;   37: jne     0x3d
;;   3d: addq    $0x10, %rsp
;;   41: popq    %rbp
;;   42: retq
;;   43: ud2
