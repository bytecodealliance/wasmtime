;;! target = "x86_64"
;;! test = "winch"


(module
  (table $t3 2 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)

  (func (export "set-funcref") (param $i i32) (param $r funcref)
    (table.set $t3 (local.get $i) (local.get $r))
  )
  (func (export "set-funcref-from") (param $i i32) (param $j i32)
    (table.set $t3 (local.get $i) (table.get $t3 (local.get $j)))
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
;;   55: ja      0xae
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x20, %rsp
;;   62: movq    %rdi, 0x18(%rsp)
;;   67: movq    %rsi, 0x10(%rsp)
;;   6c: movl    %edx, 0xc(%rsp)
;;   70: movq    %rcx, (%rsp)
;;   74: movq    (%rsp), %rax
;;   78: movl    0xc(%rsp), %ecx
;;   7c: movq    %r14, %rdx
;;   7f: movl    0x50(%rdx), %ebx
;;   82: cmpl    %ebx, %ecx
;;   84: jae     0xb0
;;   8a: movl    %ecx, %r11d
;;   8d: imulq   $8, %r11, %r11
;;   91: movq    0x48(%rdx), %rdx
;;   95: movq    %rdx, %rsi
;;   98: addq    %r11, %rdx
;;   9b: cmpl    %ebx, %ecx
;;   9d: cmovaeq %rsi, %rdx
;;   a1: orq     $1, %rax
;;   a5: movq    %rax, (%rdx)
;;   a8: addq    $0x20, %rsp
;;   ac: popq    %rbp
;;   ad: retq
;;   ae: ud2
;;   b0: ud2
;;
;; wasm[0]::function[2]:
;;   c0: pushq   %rbp
;;   c1: movq    %rsp, %rbp
;;   c4: movq    8(%rdi), %r11
;;   c8: movq    (%r11), %r11
;;   cb: addq    $0x20, %r11
;;   d2: cmpq    %rsp, %r11
;;   d5: ja      0x1a7
;;   db: movq    %rdi, %r14
;;   de: subq    $0x18, %rsp
;;   e2: movq    %rdi, 0x10(%rsp)
;;   e7: movq    %rsi, 8(%rsp)
;;   ec: movl    %edx, 4(%rsp)
;;   f0: movl    %ecx, (%rsp)
;;   f3: movl    4(%rsp), %r11d
;;   f8: subq    $4, %rsp
;;   fc: movl    %r11d, (%rsp)
;;  100: movl    4(%rsp), %r11d
;;  105: subq    $4, %rsp
;;  109: movl    %r11d, (%rsp)
;;  10d: movl    (%rsp), %ecx
;;  110: addq    $4, %rsp
;;  114: movq    %r14, %rdx
;;  117: movl    0x50(%rdx), %ebx
;;  11a: cmpl    %ebx, %ecx
;;  11c: jae     0x1a9
;;  122: movl    %ecx, %r11d
;;  125: imulq   $8, %r11, %r11
;;  129: movq    0x48(%rdx), %rdx
;;  12d: movq    %rdx, %rsi
;;  130: addq    %r11, %rdx
;;  133: cmpl    %ebx, %ecx
;;  135: cmovaeq %rsi, %rdx
;;  139: movq    (%rdx), %rax
;;  13c: testq   %rax, %rax
;;  13f: jne     0x16a
;;  145: subq    $4, %rsp
;;  149: movl    %ecx, (%rsp)
;;  14c: movq    %r14, %rdi
;;  14f: movl    $0, %esi
;;  154: movl    (%rsp), %edx
;;  157: callq   0x515
;;  15c: addq    $4, %rsp
;;  160: movq    0x14(%rsp), %r14
;;  165: jmp     0x16e
;;  16a: andq    $0xfffffffffffffffe, %rax
;;  16e: movl    (%rsp), %ecx
;;  171: addq    $4, %rsp
;;  175: movq    %r14, %rdx
;;  178: movl    0x50(%rdx), %ebx
;;  17b: cmpl    %ebx, %ecx
;;  17d: jae     0x1ab
;;  183: movl    %ecx, %r11d
;;  186: imulq   $8, %r11, %r11
;;  18a: movq    0x48(%rdx), %rdx
;;  18e: movq    %rdx, %rsi
;;  191: addq    %r11, %rdx
;;  194: cmpl    %ebx, %ecx
;;  196: cmovaeq %rsi, %rdx
;;  19a: orq     $1, %rax
;;  19e: movq    %rax, (%rdx)
;;  1a1: addq    $0x18, %rsp
;;  1a5: popq    %rbp
;;  1a6: retq
;;  1a7: ud2
;;  1a9: ud2
;;  1ab: ud2
