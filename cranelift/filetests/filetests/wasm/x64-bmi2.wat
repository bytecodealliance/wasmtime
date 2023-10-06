;;! target = "x86_64"
;;! compile = true
;;! settings = ["has_bmi2", "opt_level=speed", "has_avx"]

(module
  (func (export "bzhi32") (param i32 i32) (result i32)
    (i32.and
      (local.get 0)
      (i32.sub
        (i32.shl
          (i32.const 1)
          (local.get 1))
        (i32.const 1))))

  (func (export "bzhi64") (param i64 i64) (result i64)
    (i64.and
      (local.get 0)
      (i64.add
        (i64.shl
          (i64.const 1)
          (local.get 1))
        (i64.const -1))))

  (func (export "rorx32") (param i32) (result i32)
    (i32.rotr (local.get 0) (i32.const 8)))

  (func (export "rorx64") (param i64) (result i64)
    (i64.rotl (local.get 0) (i64.const 9)))

  (func (export "shlx32") (param i32 i32) (result i32)
    (i32.shl (local.get 0) (local.get 1)))
  (func (export "shlx64") (param i64 i64) (result i64)
    (i64.shl (local.get 0) (local.get 1)))

  (func (export "shrx32") (param i32 i32) (result i32)
    (i32.shr_u (local.get 0) (local.get 1)))
  (func (export "shrx64") (param i64 i64) (result i64)
    (i64.shr_u (local.get 0) (local.get 1)))

  (func (export "sarx32") (param i32 i32) (result i32)
    (i32.shr_s (local.get 0) (local.get 1)))
  (func (export "sarx64") (param i64 i64) (result i64)
    (i64.shr_s (local.get 0) (local.get 1)))
)
;; function u0:0:
;;   pushq   %rbp
;;   unwind PushFrameRegs { offset_upward_to_caller_sp: 16 }
;;   movq    %rsp, %rbp
;;   unwind DefineNewFrame { offset_upward_to_caller_sp: 16, offset_downward_to_clobbers: 0 }
;; block0:
;;   jmp     label1
;; block1:
;;   andl    %esi, $31, %esi
;;   bzhi    %edi, %esi, %eax
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
;;   andq    %rsi, $63, %rsi
;;   bzhi    %rdi, %rsi, %rax
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
;;   rorxl   $8, %edi, %eax
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
;;   rorxq   $55, %rdi, %rax
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
;;   shlx    %edi, %esi, %eax
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
;;   shlx    %rdi, %rsi, %rax
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
;;   shrx    %edi, %esi, %eax
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
;;   shrx    %rdi, %rsi, %rax
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
;;   sarx    %edi, %esi, %eax
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
;;   sarx    %rdi, %rsi, %rax
;;   movq    %rbp, %rsp
;;   popq    %rbp
;;   ret
