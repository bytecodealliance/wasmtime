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

;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   movq    %rdx, %rax
;;   notl    %eax, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:1:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   movq    8(%rdi), %r10
;;   movq    0(%r10), %r10
;;   cmpq    %rsp, %r10
;;   jnbe #trap=stk_ovf
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   andn    %edx, %ecx, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
