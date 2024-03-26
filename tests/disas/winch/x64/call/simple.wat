;;! target = "x86_64"
;;! test = "winch"

(module
  (func $main (result i32)
    (local $var i32)
    (call $product (i32.const 20) (i32.const 80))
    (local.set $var (i32.const 2))
    (local.get $var)
    (i32.div_u))

  (func $product (param i32 i32) (result i32)
    (local.get 0)
    (local.get 1)
    (i32.mul))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x8b
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: subq    $8, %rsp
;;   38: movq    %r14, %rdi
;;   3b: movq    %r14, %rsi
;;   3e: movl    $0x14, %edx
;;   43: movl    $0x50, %ecx
;;   48: callq   0x90
;;   4d: addq    $8, %rsp
;;   51: movq    0x10(%rsp), %r14
;;   56: movl    $2, %ecx
;;   5b: movl    %ecx, 4(%rsp)
;;   5f: subq    $4, %rsp
;;   63: movl    %eax, (%rsp)
;;   66: movl    8(%rsp), %r11d
;;   6b: subq    $4, %rsp
;;   6f: movl    %r11d, (%rsp)
;;   73: movl    (%rsp), %ecx
;;   76: addq    $4, %rsp
;;   7a: movl    (%rsp), %eax
;;   7d: addq    $4, %rsp
;;   81: xorl    %edx, %edx
;;   83: divl    %ecx
;;   85: addq    $0x18, %rsp
;;   89: popq    %rbp
;;   8a: retq
;;   8b: ud2
;;
;; wasm[0]::function[1]:
;;   90: pushq   %rbp
;;   91: movq    %rsp, %rbp
;;   94: movq    8(%rdi), %r11
;;   98: movq    (%r11), %r11
;;   9b: addq    $0x18, %r11
;;   a2: cmpq    %rsp, %r11
;;   a5: ja      0xd5
;;   ab: movq    %rdi, %r14
;;   ae: subq    $0x18, %rsp
;;   b2: movq    %rdi, 0x10(%rsp)
;;   b7: movq    %rsi, 8(%rsp)
;;   bc: movl    %edx, 4(%rsp)
;;   c0: movl    %ecx, (%rsp)
;;   c3: movl    (%rsp), %eax
;;   c6: movl    4(%rsp), %ecx
;;   ca: imull   %eax, %ecx
;;   cd: movl    %ecx, %eax
;;   cf: addq    $0x18, %rsp
;;   d3: popq    %rbp
;;   d4: retq
;;   d5: ud2
