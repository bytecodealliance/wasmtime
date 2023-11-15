;;! target = "x86_64"
(module
  (func (export "as-local-set-value") (result i32)
    (local i32) (local.set 0 (loop (result i32) (i32.const 1))) (local.get 0)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c893424             	mov	qword ptr [rsp], r14
;;   15:	 b801000000           	mov	eax, 1
;;   1a:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1e:	 8b44240c             	mov	eax, dword ptr [rsp + 0xc]
;;   22:	 4883c410             	add	rsp, 0x10
;;   26:	 5d                   	pop	rbp
;;   27:	 c3                   	ret	
