;;! target = "x86_64"
;;! flags = ["has_lzcnt"]

(module
    (func (result i32)
        (local $foo i32)

        (i32.const 2)
        (local.set $foo)

        (local.get $foo)
        (i32.clz)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec10             	sub	rsp, 0x10
;;      	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b802000000           	mov	eax, 2
;;      	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;      	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;      	 f30fbdc0             	lzcnt	eax, eax
;;      	 4883c410             	add	rsp, 0x10
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
