;;! target = "x86_64"

(module
    (func (result i32)
        (local $foo i32)

        (i32.const 2)
        (local.set $foo)

        (local.get $foo)
        (i32.clz)
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
;;   22:	 0fbdc0               	bsr	eax, eax
;;   25:	 41bb00000000         	mov	r11d, 0
;;   2b:	 410f95c3             	setne	r11b
;;   2f:	 f7d8                 	neg	eax
;;   31:	 83c020               	add	eax, 0x20
;;   34:	 4429d8               	sub	eax, r11d
;;   37:	 4883c410             	add	rsp, 0x10
;;   3b:	 5d                   	pop	rbp
;;   3c:	 c3                   	ret	
