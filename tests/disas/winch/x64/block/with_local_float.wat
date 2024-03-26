;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "") (param f32) (result f32)
    local.get 0
    block
    end
  )
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x1c, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x52
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movss   %xmm0, 4(%rsp)
;;   32: movss   4(%rsp), %xmm15
;;   39: subq    $4, %rsp
;;   3d: movss   %xmm15, (%rsp)
;;   43: movss   (%rsp), %xmm0
;;   48: addq    $4, %rsp
;;   4c: addq    $0x18, %rsp
;;   50: popq    %rbp
;;   51: retq
;;   52: ud2
