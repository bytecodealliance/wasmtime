;;! target = "x86_64"
;;! compile = true
;;! settings = ["has_bmi1", "opt_level=speed", "has_avx"]

(module
  (func (export "blsi32") (param i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.sub (i32.const 0) (local.get 0))))

  (func (export "blsi64") (param i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.sub (i64.const 0) (local.get 0))))

  (func (export "blsr32") (param i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.add (local.get 0) (i32.const -1))))

  (func (export "blsr64") (param i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.sub (local.get 0) (i64.const 1))))

  (func (export "blsmsk32") (param i32) (result i32)
    (i32.xor
      (local.get 0)
      (i32.sub (local.get 0) (i32.const 1))))

  (func (export "blsmsk64") (param i64) (result i64)
    (i64.xor
      (local.get 0)
      (i64.add (local.get 0) (i64.const -1))))

  (func (export "tzcnt32") (param i32) (result i32)
    (i32.ctz (local.get 0)))

  (func (export "tzcnt64") (param i64) (result i64)
    (i64.ctz (local.get 0)))

  (func (export "andn32") (param i32 i32) (result i32)
    (i32.and (local.get 0) (i32.xor (local.get 1) (i32.const -1))))

  (func (export "andn64") (param i64 i64) (result i64)
    (i64.and (local.get 0) (i64.xor (local.get 1) (i64.const -1))))
)
;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   blsil   %edi, %eax
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
;;   blsiq   %rdi, %rax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:2:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   blsrl   %edi, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:3:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   blsrq   %rdi, %rax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:4:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   blsmskl %edi, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:5:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   blsmskq %rdi, %rax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:6:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   tzcntl  %edi, %eax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:7:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   tzcntq  %rdi, %rax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
;;
;; function u0:8:
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
;;
;; function u0:9:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   andn    %rdi, %rsi, %rax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
