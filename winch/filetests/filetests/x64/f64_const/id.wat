;;! target = "x86_64"

(module
  (func (param f64) (result f64) (local.get 0))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f20f11442408         	movsd	qword ptr [rsp + 8], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f20f10442408         	movsd	xmm0, qword ptr [rsp + 8]
;;   18:	 4883c410             	add	rsp, 0x10
;;   1c:	 5d                   	pop	rbp
;;   1d:	 c3                   	ret	
