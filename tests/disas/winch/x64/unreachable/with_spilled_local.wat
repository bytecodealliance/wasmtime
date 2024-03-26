;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "")
    (local i32)
    local.get 0
    block
    end
    unreachable
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x1c, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x49
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    4(%rsp), %r11d
;;   39: subq    $4, %rsp
;;   3d: movl    %r11d, (%rsp)
;;   41: ud2
;;   43: addq    $0x18, %rsp
;;   47: popq    %rbp
;;   48: retq
;;   49: ud2
