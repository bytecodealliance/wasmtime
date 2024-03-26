;;! target = "x86_64"
;;! test = "winch"


(module
  (func (export "as-if-else") (param i32 i32) (result i32)
    (if (result i32) (local.get 0) (then (local.get 1)) (else (unreachable)))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4f
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    %ecx, (%rsp)
;;   33: movl    4(%rsp), %eax
;;   37: testl   %eax, %eax
;;   39: je      0x47
;;   3f: movl    (%rsp), %eax
;;   42: jmp     0x49
;;   47: ud2
;;   49: addq    $0x18, %rsp
;;   4d: popq    %rbp
;;   4e: retq
;;   4f: ud2
