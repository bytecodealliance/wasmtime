;;! target = "x86_64"
;;! test = "winch"
(module
  (type $t0 (func))
  (func $f1 (type $t0))
  (func $f2 (type $t0))
  (func $f3 (type $t0))

  ;; Define two tables of funcref
  (table $t1 3 funcref)
  (table $t2 10 funcref)

  ;; Initialize table $t1 with functions $f1, $f2, $f3
  (elem (i32.const 0) $f1 $f2 $f3)

  ;; Function to fill table $t1 using a function reference from table $t2
  (func (export "fill") (param $i i32) (param $r i32) (param $n i32)
    (local $ref funcref)
    (local.set $ref (table.get $t1 (local.get $r)))
    (table.fill $t2 (local.get $i) (local.get $ref) (local.get $n))
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
;;   4b: addq    $0x10, %r11
;;   52: cmpq    %rsp, %r11
;;   55: ja      0x71
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x10, %rsp
;;   62: movq    %rdi, 8(%rsp)
;;   67: movq    %rsi, (%rsp)
;;   6b: addq    $0x10, %rsp
;;   6f: popq    %rbp
;;   70: retq
;;   71: ud2
;;
;; wasm[0]::function[2]:
;;   80: pushq   %rbp
;;   81: movq    %rsp, %rbp
;;   84: movq    8(%rdi), %r11
;;   88: movq    (%r11), %r11
;;   8b: addq    $0x10, %r11
;;   92: cmpq    %rsp, %r11
;;   95: ja      0xb1
;;   9b: movq    %rdi, %r14
;;   9e: subq    $0x10, %rsp
;;   a2: movq    %rdi, 8(%rsp)
;;   a7: movq    %rsi, (%rsp)
;;   ab: addq    $0x10, %rsp
;;   af: popq    %rbp
;;   b0: retq
;;   b1: ud2
;;
;; wasm[0]::function[3]:
;;   c0: pushq   %rbp
;;   c1: movq    %rsp, %rbp
;;   c4: movq    8(%rdi), %r11
;;   c8: movq    (%r11), %r11
;;   cb: addq    $0x40, %r11
;;   d2: cmpq    %rsp, %r11
;;   d5: ja      0x1d8
;;   db: movq    %rdi, %r14
;;   de: subq    $0x28, %rsp
;;   e2: movq    %rdi, 0x20(%rsp)
;;   e7: movq    %rsi, 0x18(%rsp)
;;   ec: movl    %edx, 0x14(%rsp)
;;   f0: movl    %ecx, 0x10(%rsp)
;;   f4: movl    %r8d, 0xc(%rsp)
;;   f9: movl    $0, 8(%rsp)
;;  101: movq    $0, (%rsp)
;;  109: movl    0x10(%rsp), %r11d
;;  10e: subq    $4, %rsp
;;  112: movl    %r11d, (%rsp)
;;  116: movl    (%rsp), %ecx
;;  119: addq    $4, %rsp
;;  11d: movq    %r14, %rdx
;;  120: movl    0x50(%rdx), %ebx
;;  123: cmpl    %ebx, %ecx
;;  125: jae     0x1da
;;  12b: movl    %ecx, %r11d
;;  12e: imulq   $8, %r11, %r11
;;  132: movq    0x48(%rdx), %rdx
;;  136: movq    %rdx, %rsi
;;  139: addq    %r11, %rdx
;;  13c: cmpl    %ebx, %ecx
;;  13e: cmovaeq %rsi, %rdx
;;  142: movq    (%rdx), %rax
;;  145: testq   %rax, %rax
;;  148: jne     0x17c
;;  14e: subq    $4, %rsp
;;  152: movl    %ecx, (%rsp)
;;  155: subq    $4, %rsp
;;  159: movq    %r14, %rdi
;;  15c: movl    $0, %esi
;;  161: movl    4(%rsp), %edx
;;  165: callq   0x5ee
;;  16a: addq    $4, %rsp
;;  16e: addq    $4, %rsp
;;  172: movq    0x20(%rsp), %r14
;;  177: jmp     0x180
;;  17c: andq    $0xfffffffffffffffe, %rax
;;  180: movq    %rax, 4(%rsp)
;;  185: movl    0x14(%rsp), %r11d
;;  18a: subq    $4, %rsp
;;  18e: movl    %r11d, (%rsp)
;;  192: movq    8(%rsp), %r11
;;  197: pushq   %r11
;;  199: movl    0x18(%rsp), %r11d
;;  19e: subq    $4, %rsp
;;  1a2: movl    %r11d, (%rsp)
;;  1a6: subq    $8, %rsp
;;  1aa: movq    %r14, %rdi
;;  1ad: movl    $1, %esi
;;  1b2: movl    0x14(%rsp), %edx
;;  1b6: movq    0xc(%rsp), %rcx
;;  1bb: movl    8(%rsp), %r8d
;;  1c0: callq   0x630
;;  1c5: addq    $8, %rsp
;;  1c9: addq    $0x10, %rsp
;;  1cd: movq    0x20(%rsp), %r14
;;  1d2: addq    $0x28, %rsp
;;  1d6: popq    %rbp
;;  1d7: retq
;;  1d8: ud2
;;  1da: ud2
