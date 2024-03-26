;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "as-if-else") (param i32 i32)
    (block
      (if (local.get 0) (then (call $dummy)) (else (br_if 1 (local.get 1))))
    )
  )
)

;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x31
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: addq    $0x10, %rsp
;;   2f: popq    %rbp
;;   30: retq
;;   31: ud2
;;
;; wasm[0]::function[1]:
;;   40: pushq   %rbp
;;   41: movq    %rsp, %rbp
;;   44: movq    8(%rdi), %r11
;;   48: movq    (%r11), %r11
;;   4b: addq    $0x20, %r11
;;   52: cmpq    %rsp, %r11
;;   55: ja      0xad
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    %ecx, (%rsp)
;;   73: movl    4(%rsp), %eax
;;   77: testl   %eax, %eax
;;   79: je      0x9c
;;   7f: subq    $8, %rsp
;;   83: movq    %r14, %rdi
;;   86: movq    %r14, %rsi
;;   89: callq   0
;;   8e: addq    $8, %rsp
;;   92: movq    0x10(%rsp), %r14
;;   97: jmp     0xa7
;;   9c: movl    (%rsp), %eax
;;   9f: testl   %eax, %eax
;;   a1: jne     0xa7
;;   a7: addq    $0x18, %rsp
;;   ab: popq    %rbp
;;   ac: retq
;;   ad: ud2
