;;! target = "x86_64"
;;! test = "winch"

(module
  (func (export "")
    (local i32)
    local.get 0
    if
      local.get 0
      block
      end
      unreachable
    else
      nop
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
;;   15: ja      0x55
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    4(%rsp), %eax
;;   38: testl   %eax, %eax
;;   3a: je      0x4f
;;   40: movl    4(%rsp), %r11d
;;   45: subq    $4, %rsp
;;   49: movl    %r11d, (%rsp)
;;   4d: ud2
;;   4f: addq    $0x18, %rsp
;;   53: popq    %rbp
;;   54: retq
;;   55: ud2
