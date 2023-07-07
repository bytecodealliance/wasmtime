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
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 b802000000           	mov	eax, 2
;;   1b:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1f:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   23:	 0fbdc0               	bsr	eax, eax
;;   26:	 41bb00000000         	mov	r11d, 0
;;   2c:	 410f95c3             	setne	r11b
;;   30:	 f7d8                 	neg	eax
;;   32:	 83c020               	add	eax, 0x20
;;   35:	 4429d8               	sub	eax, r11d
;;   38:	 4883c410             	add	rsp, 0x10
;;   3c:	 5d                   	pop	rbp
;;   3d:	 c3                   	ret	
