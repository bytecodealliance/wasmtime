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
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec08             	sub	rsp, 8
;;    8:	 4c893424             	mov	qword ptr [rsp], r14
;;    c:	 b800000000           	mov	eax, 0
;;   11:	 4883ec04             	sub	rsp, 4
;;   15:	 890424               	mov	dword ptr [rsp], eax
;;   18:	 b900000000           	mov	ecx, 0
;;   1d:	 b800000000           	mov	eax, 0
;;   22:	 4883c404             	add	rsp, 4
;;   26:	 ba00000000           	mov	edx, 0
;;   2b:	 39ca                 	cmp	edx, ecx
;;   2d:	 0f42ca               	cmovb	ecx, edx
;;   30:	 4c8d1d0a000000       	lea	r11, [rip + 0xa]
;;   37:	 4963148b             	movsxd	rdx, dword ptr [r11 + rcx*4]
;;   3b:	 4901d3               	add	r11, rdx
;;   3e:	 41ffe3               	jmp	r11
;;   41:	 0400                 	add	al, 0
;;   43:	 0000                 	add	byte ptr [rax], al
;;   45:	 4883c408             	add	rsp, 8
;;   49:	 5d                   	pop	rbp
;;   4a:	 c3                   	ret	
