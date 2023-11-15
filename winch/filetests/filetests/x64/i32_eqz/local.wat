;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)

        (i32.const 2)
        (local.set $foo)

        (local.get $foo)
        (i32.eqz)
    )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b802000000           	mov	eax, 2
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   22:	 83f800               	cmp	eax, 0
;;   25:	 b800000000           	mov	eax, 0
;;   2a:	 400f94c0             	sete	al
;;   2e:	 4883c410             	add	rsp, 0x10
;;   32:	 5d                   	pop	rbp
;;   33:	 c3                   	ret	
