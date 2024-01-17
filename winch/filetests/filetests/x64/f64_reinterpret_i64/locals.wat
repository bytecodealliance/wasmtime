;;! target = "x86_64"

(module
    (func (result f64)
        (local i64)  

        (local.get 0)
        (f64.reinterpret_i64)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 488b442408           	mov	rax, qword ptr [rsp + 8]
;;      	 66480f6ec0           	movq	xmm0, rax
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
