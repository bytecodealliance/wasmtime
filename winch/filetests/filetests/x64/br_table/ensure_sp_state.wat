;;! target = "x86_64"

(module
  (func (export "") (result i32)
    block (result i32)
       i32.const 0
    end
    i32.const 0
    i32.const 0
    br_table 0
  )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8743000000         	ja	0x5b
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b800000000           	mov	eax, 0
;;      	 4883ec04             	sub	rsp, 4
;;      	 890424               	mov	dword ptr [rsp], eax
;;      	 b900000000           	mov	ecx, 0
;;      	 b800000000           	mov	eax, 0
;;      	 ba00000000           	mov	edx, 0
;;      	 39ca                 	cmp	edx, ecx
;;      	 0f42ca               	cmovb	ecx, edx
;;      	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;      	 4963148b             	movsxd	rdx, dword ptr [r11 + rcx*4]
;;      	 4901d3               	add	r11, rdx
;;      	 41ffe3               	jmp	r11
;;   4d:	 0400                 	add	al, 0
;;      	 0000                 	add	byte ptr [rax], al
;;      	 4883c404             	add	rsp, 4
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   5b:	 0f0b                 	ud2	
