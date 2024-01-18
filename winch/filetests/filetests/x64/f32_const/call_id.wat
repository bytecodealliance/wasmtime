;;! target = "x86_64"

(module
  (func $id-f32 (param f32) (result f32) (local.get 0))
  (func (export "type-first-f32") (result f32) (call $id-f32 (f32.const 1.32)))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8716000000         	ja	0x2e
;;   18:	 f30f1144240c         	movss	dword ptr [rsp + 0xc], xmm0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f1044240c         	movss	xmm0, dword ptr [rsp + 0xc]
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   2e:	 0f0b                 	ud2	
;;
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871f000000         	ja	0x37
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 4883ec08             	sub	rsp, 8
;;      	 f30f100518000000     	movss	xmm0, dword ptr [rip + 0x18]
;;      	 e800000000           	call	0x2d
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   37:	 0f0b                 	ud2	
;;   39:	 0000                 	add	byte ptr [rax], al
;;   3b:	 0000                 	add	byte ptr [rax], al
;;   3d:	 0000                 	add	byte ptr [rax], al
;;   3f:	 00c3                 	add	bl, al
;;   41:	 f5                   	cmc	
;;   42:	 a83f                 	test	al, 0x3f
