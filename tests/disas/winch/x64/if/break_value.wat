;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "break-value") (param i32) (result i32)
    (if (result i32) (local.get 0)
      (then (br 0 (i32.const 18)) (i32.const 19))
      (else (br 0 (i32.const 21)) (i32.const 20))
    )
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x51
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %eax
;;   34: testl   %eax, %eax
;;   36: je      0x46
;;   3c: movl    $0x12, %eax
;;   41: jmp     0x4b
;;   46: movl    $0x15, %eax
;;   4b: addq    $0x18, %rsp
;;   4f: popq    %rbp
;;   50: retq
;;   51: ud2
