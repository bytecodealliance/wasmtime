;;! target = "x86_64"

(module
  (func $id-f32 (param f32) (result f32) (local.get 0))
  (func (export "type-first-f32") (result f32) (call $id-f32 (f32.const 1.32)))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883ec08             	sub	rsp, 8
;;      	 f30f100510000000     	movss	xmm0, dword ptr [rip + 0x10]
;;      	 e800000000           	call	0x1d
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   27:	 00c3                 	add	bl, al
;;   29:	 f5                   	cmc	
;;   2a:	 a83f                 	test	al, 0x3f
