;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (param i32) (result i32)
    local.get 0
    block
    end
    local.tee 0
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x1c, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x4e
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movl    %edx, 4(%rsp)
;;   30: movl    4(%rsp), %r11d
;;   35: subq    $4, %rsp
;;   39: movl    %r11d, (%rsp)
;;   3d: movl    (%rsp), %eax
;;   40: addq    $4, %rsp
;;   44: movl    %eax, 4(%rsp)
;;   48: addq    $0x18, %rsp
;;   4c: popq    %rbp
;;   4d: retq
;;   4e: ud2
