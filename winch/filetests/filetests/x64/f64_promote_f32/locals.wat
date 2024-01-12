;;! target = "x86_64"

(module
    (func (result f64)
        (local f32)  

        (local.get 0)
        (f64.promote_f32)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   1b:	 f30f5ac0             	cvtss2sd	xmm0, xmm0
;;   1f:	 4883c410             	add	rsp, 0x10
;;   23:	 5d                   	pop	rbp
;;   24:	 c3                   	ret	
