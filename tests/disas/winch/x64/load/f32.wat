;;! target = "x86_64"
;;! test = "winch"

(module
  (memory (data "\00\00\a0\7f"))

  (func (export "f32.load") (result f32) (f32.load (i32.const 0)))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x10, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x41
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x10, %rsp
;;   22: movq    %rdi, 8(%rsp)
;;   27: movq    %rsi, (%rsp)
;;   2b: movl    $0, %eax
;;   30: movq    0x50(%r14), %rcx
;;   34: addq    %rax, %rcx
;;   37: movss   (%rcx), %xmm0
;;   3b: addq    $0x10, %rsp
;;   3f: popq    %rbp
;;   40: retq
;;   41: ud2
