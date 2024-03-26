;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "as-if-then-no-else") (param i32 i32) (result i32)
    (if (local.get 0) (then (unreachable))) (local.get 1)
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4a
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    %ecx, (%rsp)
;;   33: movl    4(%rsp), %eax
;;   37: testl   %eax, %eax
;;   39: je      0x41
;;   3f: ud2
;;   41: movl    (%rsp), %eax
;;   44: addq    $0x18, %rsp
;;   48: popq    %rbp
;;   49: retq
;;   4a: ud2
