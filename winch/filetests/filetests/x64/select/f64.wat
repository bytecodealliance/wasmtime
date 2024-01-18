;;! target = "x86_64"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec20             	sub	rsp, 0x20
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8737000000         	ja	0x4f
;;   18:	 f20f11442418         	movsd	qword ptr [rsp + 0x18], xmm0
;;      	 f20f114c2410         	movsd	qword ptr [rsp + 0x10], xmm1
;;      	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 f20f10442410         	movsd	xmm0, qword ptr [rsp + 0x10]
;;      	 f20f104c2418         	movsd	xmm1, qword ptr [rsp + 0x18]
;;      	 83f800               	cmp	eax, 0
;;      	 0f8404000000         	je	0x49
;;   45:	 f20f10c1             	movsd	xmm0, xmm1
;;      	 4883c420             	add	rsp, 0x20
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   4f:	 0f0b                 	ud2	
