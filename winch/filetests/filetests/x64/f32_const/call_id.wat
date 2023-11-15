;;! target = "x86_64"

(module
  (func $id-f32 (param f32) (result f32) (local.get 0))
  (func (export "type-first-f32") (result f32) (call $id-f32 (f32.const 1.32)))
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;    e:	 4c893424             	mov	qword ptr [rsp], r14
;;   12:	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;   18:	 4883c410             	add	rsp, 0x10
;;   1c:	 5d                   	pop	rbp
;;   1d:	 c3                   	ret	
;;
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 4883ec08             	sub	rsp, 8
;;   10:	 f30f100510000000     	movss	xmm0, dword ptr [rip + 0x10]
;;   18:	 e800000000           	call	0x1d
;;   1d:	 4883c408             	add	rsp, 8
;;   21:	 4883c408             	add	rsp, 8
;;   25:	 5d                   	pop	rbp
;;   26:	 c3                   	ret	
;;   27:	 00c3                 	add	bl, al
;;   29:	 f5                   	cmc	
;;   2a:	 a83f                 	test	al, 0x3f
