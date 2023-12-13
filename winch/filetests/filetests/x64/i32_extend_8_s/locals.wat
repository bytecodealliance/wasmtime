;;! target = "x86_64"

(module
    (func (result i32)
        (local i32)

        (local.get 0)
        (i32.extend8_s)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   19:	 0fbec0               	movsx	eax, al
;;   1c:	 4883c410             	add	rsp, 0x10
;;   20:	 5d                   	pop	rbp
;;   21:	 c3                   	ret	
