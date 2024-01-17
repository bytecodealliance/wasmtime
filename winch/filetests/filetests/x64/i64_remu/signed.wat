;;! target = "x86_64"

(module
    (func (result i64)
	(i64.const -1)
	(i64.const -1)
	(i64.rem_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8721000000         	ja	0x39
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c1ffffffff       	mov	rcx, 0xffffffffffffffff
;;      	 48c7c0ffffffff       	mov	rax, 0xffffffffffffffff
;;      	 4831d2               	xor	rdx, rdx
;;      	 48f7f1               	div	rcx
;;      	 4889d0               	mov	rax, rdx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   39:	 0f0b                 	ud2	
