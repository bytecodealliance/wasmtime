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
;;      	 4989fe               	mov	r14, rdi
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4981c314000000       	add	r11, 0x14
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f874c000000         	ja	0x6a
;;   1e:	 4883ec10             	sub	rsp, 0x10
;;      	 48897c2408           	mov	qword ptr [rsp + 8], rdi
;;      	 48893424             	mov	qword ptr [rsp], rsi
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
;;   5c:	 0400                 	add	al, 0
;;      	 0000                 	add	byte ptr [rax], al
;;      	 4883c404             	add	rsp, 4
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   6a:	 0f0b                 	ud2	
