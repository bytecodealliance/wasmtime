;;! target = "x86_64"
;;! test = "winch"
;;! flags = ["-W", "exceptions"]

;; Currently exceptions trap on throw: `throw` becomes a trap and
;; `try_table` compiles as a plain block.
(module
  (tag $e (param i32))
  (func (result i32)
    (block $h (result i32)
      (try_table (result i32) (catch $e $h)
        (throw $e (i32.const 42))))))
;; wasm[0]::function[0]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    8(%rdi), %r11
;;       movq    0x18(%r11), %r11
;;       addq    $0x10, %r11
;;       cmpq    %rsp, %r11
;;       ja      0x3a
;;   1c: movq    %rdi, %r14
;;       subq    $0x10, %rsp
;;       movq    %rdi, 8(%rsp)
;;       movq    %rsi, (%rsp)
;;       ud2
;;       addq    $0x10, %rsp
;;       popq    %rbp
;;       retq
;;   3a: ud2
