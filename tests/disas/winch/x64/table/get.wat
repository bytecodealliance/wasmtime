;;! target = "x86_64"
;;! test = "winch"
(module
  (table $t3 3 funcref)
  (elem (table $t3) (i32.const 1) func $dummy)
  (func $dummy)
  (func $f3 (export "get-funcref") (param $i i32) (result funcref)
    (table.get $t3 (local.get $i))
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
;;   55: ja      0xed
;;   5b: movq    %rdi, %r14
;;   5e: subq    $0x18, %rsp
;;   62: movq    %rdi, 0x10(%rsp)
;;   67: movq    %rsi, 8(%rsp)
;;   6c: movl    %edx, 4(%rsp)
;;   70: movl    4(%rsp), %r11d
;;   75: subq    $4, %rsp
;;   79: movl    %r11d, (%rsp)
;;   7d: movl    (%rsp), %ecx
;;   80: addq    $4, %rsp
;;   84: movq    %r14, %rdx
;;   87: movl    0x50(%rdx), %ebx
;;   8a: cmpl    %ebx, %ecx
;;   8c: jae     0xef
;;   92: movl    %ecx, %r11d
;;   95: imulq   $8, %r11, %r11
;;   99: movq    0x48(%rdx), %rdx
;;   9d: movq    %rdx, %rsi
;;   a0: addq    %r11, %rdx
;;   a3: cmpl    %ebx, %ecx
;;   a5: cmovaeq %rsi, %rdx
;;   a9: movq    (%rdx), %rax
;;   ac: testq   %rax, %rax
;;   af: jne     0xe3
;;   b5: subq    $4, %rsp
;;   b9: movl    %ecx, (%rsp)
;;   bc: subq    $4, %rsp
;;   c0: movq    %r14, %rdi
;;   c3: movl    $0, %esi
;;   c8: movl    4(%rsp), %edx
;;   cc: callq   0x33d
;;   d1: addq    $4, %rsp
;;   d5: addq    $4, %rsp
;;   d9: movq    0x10(%rsp), %r14
;;   de: jmp     0xe7
;;   e3: andq    $0xfffffffffffffffe, %rax
;;   e7: addq    $0x18, %rsp
;;   eb: popq    %rbp
;;   ec: retq
;;   ed: ud2
;;   ef: ud2
