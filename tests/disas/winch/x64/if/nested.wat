;;! target = "x86_64"
;;! test = "winch"
(module
  (func $dummy)
  (func (export "nested") (param i32 i32) (result i32)
    (if (result i32) (local.get 0)
      (then
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 9))
          (else (call $dummy) (i32.const 10))
        )
      )
      (else
        (if (local.get 1) (then (call $dummy) (nop)))
        (if (local.get 1) (then) (else (call $dummy) (nop)))
        (if (result i32) (local.get 1)
          (then (call $dummy) (i32.const 10))
          (else (call $dummy) (i32.const 11))
        )
      )
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
;;   55: ja      0x1b4
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    %ecx, (%rsp)
;;   73: movl    4(%rsp), %eax
;;   77: testl   %eax, %eax
;;   79: je      0x119
;;   7f: movl    (%rsp), %eax
;;   82: testl   %eax, %eax
;;   84: je      0xa2
;;   8a: subq    $8, %rsp
;;   8e: movq    %r14, %rdi
;;   91: movq    %r14, %rsi
;;   94: callq   0
;;   99: addq    $8, %rsp
;;   9d: movq    0x10(%rsp), %r14
;;   a2: movl    (%rsp), %eax
;;   a5: testl   %eax, %eax
;;   a7: je      0xb2
;;   ad: jmp     0xca
;;   b2: subq    $8, %rsp
;;   b6: movq    %r14, %rdi
;;   b9: movq    %r14, %rsi
;;   bc: callq   0
;;   c1: addq    $8, %rsp
;;   c5: movq    0x10(%rsp), %r14
;;   ca: movl    (%rsp), %eax
;;   cd: testl   %eax, %eax
;;   cf: je      0xf7
;;   d5: subq    $8, %rsp
;;   d9: movq    %r14, %rdi
;;   dc: movq    %r14, %rsi
;;   df: callq   0
;;   e4: addq    $8, %rsp
;;   e8: movq    0x10(%rsp), %r14
;;   ed: movl    $9, %eax
;;   f2: jmp     0x1ae
;;   f7: subq    $8, %rsp
;;   fb: movq    %r14, %rdi
;;   fe: movq    %r14, %rsi
;;  101: callq   0
;;  106: addq    $8, %rsp
;;  10a: movq    0x10(%rsp), %r14
;;  10f: movl    $0xa, %eax
;;  114: jmp     0x1ae
;;  119: movl    (%rsp), %eax
;;  11c: testl   %eax, %eax
;;  11e: je      0x13c
;;  124: subq    $8, %rsp
;;  128: movq    %r14, %rdi
;;  12b: movq    %r14, %rsi
;;  12e: callq   0
;;  133: addq    $8, %rsp
;;  137: movq    0x10(%rsp), %r14
;;  13c: movl    (%rsp), %eax
;;  13f: testl   %eax, %eax
;;  141: je      0x14c
;;  147: jmp     0x164
;;  14c: subq    $8, %rsp
;;  150: movq    %r14, %rdi
;;  153: movq    %r14, %rsi
;;  156: callq   0
;;  15b: addq    $8, %rsp
;;  15f: movq    0x10(%rsp), %r14
;;  164: movl    (%rsp), %eax
;;  167: testl   %eax, %eax
;;  169: je      0x191
;;  16f: subq    $8, %rsp
;;  173: movq    %r14, %rdi
;;  176: movq    %r14, %rsi
;;  179: callq   0
;;  17e: addq    $8, %rsp
;;  182: movq    0x10(%rsp), %r14
;;  187: movl    $0xa, %eax
;;  18c: jmp     0x1ae
;;  191: subq    $8, %rsp
;;  195: movq    %r14, %rdi
;;  198: movq    %r14, %rsi
;;  19b: callq   0
;;  1a0: addq    $8, %rsp
;;  1a4: movq    0x10(%rsp), %r14
;;  1a9: movl    $0xb, %eax
;;  1ae: addq    $0x18, %rsp
;;  1b2: popq    %rbp
;;  1b3: retq
;;  1b4: ud2
