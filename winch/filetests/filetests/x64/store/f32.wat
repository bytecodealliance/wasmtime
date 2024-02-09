;;! target = "x86_64"

(module
  (memory (data "\00\00\a0\7f"))
  (func (export "f32.store") (f32.store (i32.const 0) (f32.const nan:0x200000)))
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8722000000         	ja	0x3a
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 f30f10051c000000     	movss	xmm0, dword ptr [rip + 0x1c]
;;      	 b800000000           	mov	eax, 0
;;      	 498b4e50             	mov	rcx, qword ptr [r14 + 0x50]
;;      	 4801c1               	add	rcx, rax
;;      	 f30f1101             	movss	dword ptr [rcx], xmm0
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3a:	 0f0b                 	ud2	
;;   3c:	 0000                 	add	byte ptr [rax], al
;;   3e:	 0000                 	add	byte ptr [rax], al
;;   40:	 0000                 	add	byte ptr [rax], al
