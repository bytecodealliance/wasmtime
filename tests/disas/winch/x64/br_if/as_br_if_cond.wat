;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-br-if-cond")
    (block (br_if 0 (br_if 0 (i32.const 1) (i32.const 1))))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $1, %eax
;;   30: testl   %eax, %eax
;;   32: jne     0x45
;;   38: movl    $1, %eax
;;   3d: testl   %eax, %eax
;;   3f: jne     0x45
;;   45: addq    $0x10, %rsp
;;   49: popq    %rbp
;;   4a: retq
;;   4b: ud2
