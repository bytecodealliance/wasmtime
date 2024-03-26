;;! target = "x86_64"
;;! test = "winch"
(module
  (memory 1)
  (func (export "i64_load8_s") (param $i i64) (result i64)
   (i64.store8 (i32.const 8) (local.get $i))
   (i64.load8_s (i32.const 8))
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x18, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x58
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    %rdx, (%rsp)
;;   30: movq    (%rsp), %rax
;;   34: movl    $8, %ecx
;;   39: movq    0x50(%r14), %rdx
;;   3d: addq    %rcx, %rdx
;;   40: movb    %al, (%rdx)
;;   42: movl    $8, %eax
;;   47: movq    0x50(%r14), %rcx
;;   4b: addq    %rax, %rcx
;;   4e: movsbq  (%rcx), %rax
;;   52: addq    $0x18, %rsp
;;   56: popq    %rbp
;;   57: retq
;;   58: ud2
