;;! target = "x86_64"
;;! test = "winch"
(module
  (func $fx (export "effects") (result i32)
    (local i32)
    (block
      (loop
        (local.set 0 (i32.const 1))
        (local.set 0 (i32.mul (local.get 0) (i32.const 3)))
        (local.set 0 (i32.sub (local.get 0) (i32.const 5)))
        (local.set 0 (i32.mul (local.get 0) (i32.const 7)))
        (br 1)
        (local.set 0 (i32.mul (local.get 0) (i32.const 100)))
      )
    )
    (i32.eq (local.get 0) (i32.const -14))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x74
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    $1, %eax
;;   39: movl    %eax, 4(%rsp)
;;   3d: movl    4(%rsp), %eax
;;   41: imull   $3, %eax, %eax
;;   44: movl    %eax, 4(%rsp)
;;   48: movl    4(%rsp), %eax
;;   4c: subl    $5, %eax
;;   4f: movl    %eax, 4(%rsp)
;;   53: movl    4(%rsp), %eax
;;   57: imull   $7, %eax, %eax
;;   5a: movl    %eax, 4(%rsp)
;;   5e: movl    4(%rsp), %eax
;;   62: cmpl    $-0xe, %eax
;;   65: movl    $0, %eax
;;   6a: sete    %al
;;   6e: addq    $0x18, %rsp
;;   72: popq    %rbp
;;   73: retq
;;   74: ud2
