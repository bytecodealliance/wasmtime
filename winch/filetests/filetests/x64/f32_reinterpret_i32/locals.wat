;;! target = "x86_64"

(module
    (func (result f32)
        (local i32)  

        (local.get 0)
        (f32.reinterpret_i32)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   19:	 660f6ec0             	movd	xmm0, eax
;;   1d:	 4883c410             	add	rsp, 0x10
;;   21:	 5d                   	pop	rbp
;;   22:	 c3                   	ret	
