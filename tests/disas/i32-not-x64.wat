;;!target = "x86_64"
;;!compile = true
;;!settings = ["opt_level=speed", "has_bmi1=true"]

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
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   movq    %rdi, %rax
;;   notl    %eax, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:1:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   andn    %edi, %esi, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
