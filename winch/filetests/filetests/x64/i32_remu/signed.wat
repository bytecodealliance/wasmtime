;;! target = "x86_64"

(module
    (func (result i32)
	(i32.const -1)
	(i32.const -1)
	(i32.rem_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f871a000000         	ja	0x32
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b9ffffffff           	mov	ecx, 0xffffffff
;;      	 b8ffffffff           	mov	eax, 0xffffffff
;;      	 31d2                 	xor	edx, edx
;;      	 f7f1                 	div	ecx
;;      	 89d0                 	mov	eax, edx
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   32:	 0f0b                 	ud2	
