;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "main") (param i32) (param i32) (result i32)
    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 1) (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))

    (local.get 1)
    (local.get 0)
    (i32.div_u)

    (call $add (i32.const 2) (i32.const 3) (i32.const 4) (i32.const 5) (i32.const 6) (i32.const 7) (i32.const 8))
  )

  (func $add (param i32 i32 i32 i32 i32 i32 i32 i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.add)
    (local.get 2)
    (i32.add)
    (local.get 3)
    (i32.add)
    (local.get 4)
    (i32.add)
    (local.get 5)
    (i32.add)
    (local.get 6)
    (i32.add)
    (local.get 7)
    (i32.add)
    (local.get 8)
    (i32.add)
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x50, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x152
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    %ecx, (%rsp)
;;   33: movl    4(%rsp), %ecx
;;   37: movl    (%rsp), %eax
;;   3a: xorl    %edx, %edx
;;   3c: divl    %ecx
;;   3e: subq    $4, %rsp
;;   42: movl    %eax, (%rsp)
;;   45: subq    $0x34, %rsp
;;   49: movq    %r14, %rdi
;;   4c: movq    %r14, %rsi
;;   4f: movl    0x34(%rsp), %edx
;;   53: movl    $1, %ecx
;;   58: movl    $2, %r8d
;;   5e: movl    $3, %r9d
;;   64: movl    $4, %r11d
;;   6a: movl    %r11d, (%rsp)
;;   6e: movl    $5, %r11d
;;   74: movl    %r11d, 8(%rsp)
;;   79: movl    $6, %r11d
;;   7f: movl    %r11d, 0x10(%rsp)
;;   84: movl    $7, %r11d
;;   8a: movl    %r11d, 0x18(%rsp)
;;   8f: movl    $8, %r11d
;;   95: movl    %r11d, 0x20(%rsp)
;;   9a: callq   0x160
;;   9f: addq    $0x34, %rsp
;;   a3: addq    $4, %rsp
;;   a7: movq    0x10(%rsp), %r14
;;   ac: subq    $4, %rsp
;;   b0: movl    %eax, (%rsp)
;;   b3: movl    4(%rsp), %r11d
;;   b8: subq    $4, %rsp
;;   bc: movl    %r11d, (%rsp)
;;   c0: movl    0xc(%rsp), %r11d
;;   c5: subq    $4, %rsp
;;   c9: movl    %r11d, (%rsp)
;;   cd: movl    (%rsp), %ecx
;;   d0: addq    $4, %rsp
;;   d4: movl    (%rsp), %eax
;;   d7: addq    $4, %rsp
;;   db: xorl    %edx, %edx
;;   dd: divl    %ecx
;;   df: subq    $4, %rsp
;;   e3: movl    %eax, (%rsp)
;;   e6: subq    $0x30, %rsp
;;   ea: movq    %r14, %rdi
;;   ed: movq    %r14, %rsi
;;   f0: movl    0x34(%rsp), %edx
;;   f4: movl    0x30(%rsp), %ecx
;;   f8: movl    $2, %r8d
;;   fe: movl    $3, %r9d
;;  104: movl    $4, %r11d
;;  10a: movl    %r11d, (%rsp)
;;  10e: movl    $5, %r11d
;;  114: movl    %r11d, 8(%rsp)
;;  119: movl    $6, %r11d
;;  11f: movl    %r11d, 0x10(%rsp)
;;  124: movl    $7, %r11d
;;  12a: movl    %r11d, 0x18(%rsp)
;;  12f: movl    $8, %r11d
;;  135: movl    %r11d, 0x20(%rsp)
;;  13a: callq   0x160
;;  13f: addq    $0x30, %rsp
;;  143: addq    $8, %rsp
;;  147: movq    0x10(%rsp), %r14
;;  14c: addq    $0x18, %rsp
;;  150: popq    %rbp
;;  151: retq
;;  152: ud2
;;
;; wasm[0]::function[1]:
;;  160: pushq   %rbp
;;  161: movq    %rsp, %rbp
;;  164: movq    8(%rdi), %r11
;;  168: movq    (%r11), %r11
;;  16b: addq    $0x20, %r11
;;  172: cmpq    %rsp, %r11
;;  175: ja      0x1d3
;;  17b: movq    %rdi, %r14
;;  17e: subq    $0x20, %rsp
;;  182: movq    %rdi, 0x18(%rsp)
;;  187: movq    %rsi, 0x10(%rsp)
;;  18c: movl    %edx, 0xc(%rsp)
;;  190: movl    %ecx, 8(%rsp)
;;  194: movl    %r8d, 4(%rsp)
;;  199: movl    %r9d, (%rsp)
;;  19d: movl    8(%rsp), %eax
;;  1a1: movl    0xc(%rsp), %ecx
;;  1a5: addl    %eax, %ecx
;;  1a7: movl    4(%rsp), %eax
;;  1ab: addl    %eax, %ecx
;;  1ad: movl    (%rsp), %eax
;;  1b0: addl    %eax, %ecx
;;  1b2: movl    0x10(%rbp), %eax
;;  1b5: addl    %eax, %ecx
;;  1b7: movl    0x18(%rbp), %eax
;;  1ba: addl    %eax, %ecx
;;  1bc: movl    0x20(%rbp), %eax
;;  1bf: addl    %eax, %ecx
;;  1c1: movl    0x28(%rbp), %eax
;;  1c4: addl    %eax, %ecx
;;  1c6: movl    0x30(%rbp), %eax
;;  1c9: addl    %eax, %ecx
;;  1cb: movl    %ecx, %eax
;;  1cd: addq    $0x20, %rsp
;;  1d1: popq    %rbp
;;  1d2: retq
;;  1d3: ud2
