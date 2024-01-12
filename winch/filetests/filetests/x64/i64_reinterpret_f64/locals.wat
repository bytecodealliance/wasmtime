;;! target = "x86_64"

(module
    (func (result i64)
        (local f64)  

        (local.get 0)
        (i64.reinterpret_f64)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   1b:	 66480f7ec0           	movq	rax, xmm0
;;   20:	 4883c410             	add	rsp, 0x10
;;   24:	 5d                   	pop	rbp
;;   25:	 c3                   	ret	
