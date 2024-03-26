;;! target = "x86_64"
;;! test = "winch"
(module
  (func (;0;) (result i32)
    (local i32)
    local.get 0
    loop ;; label = @1
      local.get 0
      block ;; label = @2
      end
      br 0 (;@1;)
    end
  )
  (export "" (func 0))
)
;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    8(%rdi), %r11
;;    8: movq    (%r11), %r11
;;    b: addq    $0x20, %r11
;;   12: cmpq    %rsp, %r11
;;   15: ja      0x5d
;;   1b: movq    %rdi, %r14
;;   1e: subq    $0x18, %rsp
;;   22: movq    %rdi, 0x10(%rsp)
;;   27: movq    %rsi, 8(%rsp)
;;   2c: movq    $0, (%rsp)
;;   34: movl    4(%rsp), %r11d
;;   39: subq    $4, %rsp
;;   3d: movl    %r11d, (%rsp)
;;   41: movl    8(%rsp), %r11d
;;   46: subq    $4, %rsp
;;   4a: movl    %r11d, (%rsp)
;;   4e: addq    $4, %rsp
;;   52: jmp     0x41
;;   57: addq    $0x18, %rsp
;;   5b: popq    %rbp
;;   5c: retq
;;   5d: ud2
