;;! target = "x86_64"

(module
  (func (export "select-f32") (param f32 f32 i32) (result f32)
    (select (local.get 0) (local.get 1) (local.get 2))
  )
)
 
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec18             	sub	rsp, 0x18
;;    8:	 f30f11442414         	movss	dword ptr [rsp + 0x14], xmm0
;;    e:	 f30f114c2410         	movss	dword ptr [rsp + 0x10], xmm1
;;   14:	 897c240c             	mov	dword ptr [rsp + 0xc], edi
;;   18:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   1d:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   21:	 f30f10442410         	movss	xmm0, dword ptr [rsp + 0x10]
;;   27:	 f30f104c2414         	movss	xmm1, dword ptr [rsp + 0x14]
;;   2d:	 85c0                 	test	eax, eax
;;   2f:	 0f8403000000         	je	0x38
;;   35:	 0f28c1               	movaps	xmm0, xmm1
;;   38:	 4883c418             	add	rsp, 0x18
;;   3c:	 5d                   	pop	rbp
;;   3d:	 c3                   	ret	
