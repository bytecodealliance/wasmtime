;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "as-local-set-value") (result i32)
    (local i32) (local.set 0 (loop (result i32) (i32.const 1))) (local.get 0)
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x47
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    $1, %eax
;;   39: movl    %eax, 4(%rsp)
;;   3d: movl    4(%rsp), %eax
;;   41: addq    $0x18, %rsp
;;   45: popq    %rbp
;;   46: retq
;;   47: ud2
