;;! target = "x86_64"
;;! test = "winch"
(module
  (func (export "break-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (br 2 (i32.const 0x1)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (loop (result i32) (br 2 (i32.const 0x2)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (block (result i32) (loop (result i32) (br 1 (i32.const 0x4))))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (br 1 (i32.const 0x8)))))))
    (local.set 0 (i32.add (local.get 0) (block (result i32) (loop (result i32) (i32.ctz (loop (result i32) (br 2 (i32.const 0x10))))))))
    (local.get 0)
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x1c, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0xe2
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    $0, %eax
;;   39: movl    %eax, 4(%rsp)
;;   3d: movl    4(%rsp), %r11d
;;   42: subq    $4, %rsp
;;   46: movl    %r11d, (%rsp)
;;   4a: movl    $1, %eax
;;   4f: movl    (%rsp), %ecx
;;   52: addq    $4, %rsp
;;   56: addl    %eax, %ecx
;;   58: movl    %ecx, 4(%rsp)
;;   5c: movl    4(%rsp), %r11d
;;   61: subq    $4, %rsp
;;   65: movl    %r11d, (%rsp)
;;   69: movl    $2, %eax
;;   6e: movl    (%rsp), %ecx
;;   71: addq    $4, %rsp
;;   75: addl    %eax, %ecx
;;   77: movl    %ecx, 4(%rsp)
;;   7b: movl    4(%rsp), %r11d
;;   80: subq    $4, %rsp
;;   84: movl    %r11d, (%rsp)
;;   88: movl    $4, %eax
;;   8d: movl    (%rsp), %ecx
;;   90: addq    $4, %rsp
;;   94: addl    %eax, %ecx
;;   96: movl    %ecx, 4(%rsp)
;;   9a: movl    4(%rsp), %r11d
;;   9f: subq    $4, %rsp
;;   a3: movl    %r11d, (%rsp)
;;   a7: movl    $8, %eax
;;   ac: movl    (%rsp), %ecx
;;   af: addq    $4, %rsp
;;   b3: addl    %eax, %ecx
;;   b5: movl    %ecx, 4(%rsp)
;;   b9: movl    4(%rsp), %r11d
;;   be: subq    $4, %rsp
;;   c2: movl    %r11d, (%rsp)
;;   c6: movl    $0x10, %eax
;;   cb: movl    (%rsp), %ecx
;;   ce: addq    $4, %rsp
;;   d2: addl    %eax, %ecx
;;   d4: movl    %ecx, 4(%rsp)
;;   d8: movl    4(%rsp), %eax
;;   dc: addq    $0x18, %rsp
;;   e0: popq    %rbp
;;   e1: retq
;;   e2: ud2
