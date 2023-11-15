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
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;   1c:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   20:	 f30f10442410         	movss	xmm0, dword ptr [rsp + 0x10]
;;   26:	 f30f104c2414         	movss	xmm1, dword ptr [rsp + 0x14]
;;   2c:	 83f800               	cmp	eax, 0
;;   2f:	 0f8404000000         	je	0x39
;;   35:	 f20f10c1             	movsd	xmm0, xmm1
;;   39:	 4883c418             	add	rsp, 0x18
;;   3d:	 5d                   	pop	rbp
;;   3e:	 c3                   	ret	
