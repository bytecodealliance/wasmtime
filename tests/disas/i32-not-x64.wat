;;! target = "x86_64"
;;! test = "compile"
;;! flags = "-C cranelift-has-bmi1"

(module
  ;; this should get optimized to a `bnot` in clif
  (func (param i32) (result i32)
    i32.const -1
    local.get 0
    i32.xor)

  ;; this should get optimized to a single `andn` instruction
  (func (param i32 i32) (result i32)
    local.get 0
    i32.const -1
    local.get 1
    i32.xor
    i32.and)
)

;; wasm[0]::function[0]:
;;    0: pushq   %rbp
;;    1: movq    %rsp, %rbp
;;    4: movq    %rdx, %rax
;;    7: notl    %eax
;;    9: movq    %rbp, %rsp
;;    c: popq    %rbp
;;    d: retq
;;
;; wasm[0]::function[1]:
;;   10: pushq   %rbp
;;   11: movq    %rsp, %rbp
;;   14: andnl   %edx, %ecx, %eax
;;   19: movq    %rbp, %rsp
;;   1c: popq    %rbp
;;   1d: retq
