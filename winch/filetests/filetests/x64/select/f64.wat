;;! target = "x86_64"

(module
  (func (export "select-f64") (param f64 f64 i32) (result f64)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec20             	sub	rsp, 0x20
;;    8:	 f20f11442418         	movsd	qword ptr [rsp + 0x18], xmm0
;;    e:	 f20f114c2410         	movsd	qword ptr [rsp + 0x10], xmm1
;;   14:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;   18:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   1d:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   21:	 f20f10442410         	movsd	xmm0, qword ptr [rsp + 0x10]
;;   27:	 f20f104c2418         	movsd	xmm1, qword ptr [rsp + 0x18]
;;   2d:	 83f800               	cmp	eax, 0
;;   30:	 0f8404000000         	je	0x3a
;;   36:	 f20f10c1             	movsd	xmm0, xmm1
;;   3a:	 4883c420             	add	rsp, 0x20
;;   3e:	 5d                   	pop	rbp
;;   3f:	 c3                   	ret	
