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
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       movq    %rdx, %rax
;;       notl    %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
;;
;; wasm[0]::function[1]:
;;       pushq   %rbp
;;       movq    %rsp, %rbp
;;       andnl   %edx, %ecx, %eax
;;       movq    %rbp, %rsp
;;       popq    %rbp
;;       retq
