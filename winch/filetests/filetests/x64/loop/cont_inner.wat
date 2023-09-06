;;! target = "x86_64"
(module
  (func (export "cont-inner") (result i32)
    (local i32)
    (local.set 0 (i32.const 0))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (loop (result i32) (br 1)))))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (i32.ctz (br 0)))))
    (local.set 0 (i32.add (local.get 0) (loop (result i32) (i32.ctz (loop (result i32) (br 1))))))
    (local.get 0)
  )
)
;;    0:	 55                   	push	rbp
;;    1:	 4889e5               	mov	rbp, rsp
;;    4:	 4883ec10             	sub	rsp, 0x10
;;    8:	 48c744240800000000   	
;; 				mov	qword ptr [rsp + 8], 0
;;   11:	 4c89742404           	mov	qword ptr [rsp + 4], r14
;;   16:	 b800000000           	mov	eax, 0
;;   1b:	 8944240c             	mov	dword ptr [rsp + 0xc], eax
;;   1f:	 448b5c240c           	mov	r11d, dword ptr [rsp + 0xc]
;;   24:	 4153                 	push	r11
;;   26:	 e9fbffffff           	jmp	0x26
;;   2b:	 4883c408             	add	rsp, 8
;;   2f:	 4883c410             	add	rsp, 0x10
;;   33:	 5d                   	pop	rbp
;;   34:	 c3                   	ret	
