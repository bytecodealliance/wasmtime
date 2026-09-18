;;! multi_memory = true

;; basic function lifting
(component
  (core module $m
    (func (export ""))
  )
  (core instance $i (instantiate $m))

  (func (export "thunk")
    (canon lift (core func $i ""))
  )
)

;; use an aliased type
(component $c
  (core module $m
    (func (export ""))
  )
  (core instance $i (instantiate $m))

  (type $to_alias (func))
  (alias outer $c $to_alias (type $alias))

  (func (export "thunk") (type $alias)
    (canon lift (core func $i ""))
  )
)

;; test out some various canonical abi
(component $c
  (core module $m
    (func (export "") (param i32 i32))
    (memory (export "memory") 1)
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      unreachable)
  )
  (core instance $i (instantiate $m))

  (func (export "thunk") (param "a" string)
    (canon lift
      (core func $i "")
      (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))
    )
  )

  (func (export "thunk8") (param "a" string)
    (canon lift
      (core func $i "")
      string-encoding=utf8
      (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))
    )
  )

  (func (export "thunk16") (param "a" string)
    (canon lift
      (core func $i "")
      string-encoding=utf16
      (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))
    )
  )

  (func (export "thunklatin16") (param "a" string)
    (canon lift
      (core func $i "")
      string-encoding=latin1+utf16
      (memory (core memory $i "memory"))
      (realloc (core func $i "realloc"))
    )
  )
)

;; lower something then immediately lift it
(component $c
  (import "host-return-two" (func $f (result u32)))

  (core func $f_lower
    (canon lower (func $f))
  )
  (func $f2 (result s32)
    (canon lift (core func $f_lower))
  )
  (export "f" (func $f2))
)

;; valid, but odd
(component
  (core module $m (func (export "")))
  (core instance $m (instantiate $m))

  (func $f1 (canon lift (core func $m "")))
  (core func $f2 (canon lower (func $f1)))
)

(component
  (core module $m (func (export "")))
  (core instance $m (instantiate $m))

  (func $f1 (canon lift (core func $m "")))
  (core func $f2 (canon lower (func $f1)))

  (core module $m2
    (import "" "" (func $f))
    (func $start
      call $f)
    (start $start)
  )
  (core instance (instantiate $m2
    (with "" (instance (export "" (func $f2))))
  ))
)

;; fiddling with 0-sized lists
(component $c
  (core module $m
    (func (export "x") (param i32 i32))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      i32.const -1)
    (memory (export "memory") 0)
  )
  (core instance $m (instantiate $m))
  (type $t' (result))
  (export $t "t" (type $t'))
  (func $f (param "a" (list $t))
    (canon lift
      (core func $m "x")
      (realloc (core func $m "realloc"))
      (memory (core memory $m "memory"))
    )
  )
  (export "empty-list" (func $f))
)
(assert_trap (invoke "empty-list" (list.const)) "realloc return: beyond end of memory")

;; Huge variants don't overflow in the FACT adapter

(component
  (type $original (variant (case "a0") (case "a1") (case "a2") (case "a3") (case "a4") (case "a5") (case "a6") (case "a7") (case "a8") (case "a9") (case "a10") (case "a11") (case "a12") (case "a13") (case "a14") (case "a15") (case "a16") (case "a17") (case "a18") (case "a19") (case "a20") (case "a21") (case "a22") (case "a23") (case "a24") (case "a25") (case "a26") (case "a27") (case "a28") (case "a29") (case "a30") (case "a31") (case "a32") (case "a33") (case "a34") (case "a35") (case "a36") (case "a37") (case "a38") (case "a39") (case "a40") (case "a41") (case "a42") (case "a43") (case "a44") (case "a45") (case "a46") (case "a47") (case "a48") (case "a49") (case "a50") (case "a51") (case "a52") (case "a53") (case "a54") (case "a55") (case "a56") (case "a57") (case "a58") (case "a59") (case "a60") (case "a61") (case "a62") (case "a63") (case "a64") (case "a65") (case "a66") (case "a67") (case "a68") (case "a69") (case "a70") (case "a71") (case "a72") (case "a73") (case "a74") (case "a75") (case "a76") (case "a77") (case "a78") (case "a79") (case "a80") (case "a81") (case "a82") (case "a83") (case "a84") (case "a85") (case "a86") (case "a87") (case "a88") (case "a89") (case "a90") (case "a91") (case "a92") (case "a93") (case "a94") (case "a95") (case "a96") (case "a97") (case "a98") (case "a99") (case "a100") (case "a101") (case "a102") (case "a103") (case "a104") (case "a105") (case "a106") (case "a107") (case "a108") (case "a109") (case "a110") (case "a111") (case "a112") (case "a113") (case "a114") (case "a115") (case "a116") (case "a117") (case "a118") (case "a119") (case "a120") (case "a121") (case "a122") (case "a123") (case "a124") (case "a125") (case "a126") (case "a127") (case "a128") (case "a129") (case "a130") (case "a131") (case "a132") (case "a133") (case "a134") (case "a135") (case "a136") (case "a137") (case "a138") (case "a139") (case "a140") (case "a141") (case "a142") (case "a143") (case "a144") (case "a145") (case "a146") (case "a147") (case "a148") (case "a149") (case "a150") (case "a151") (case "a152") (case "a153") (case "a154") (case "a155") (case "a156") (case "a157") (case "a158") (case "a159") (case "a160") (case "a161") (case "a162") (case "a163") (case "a164") (case "a165") (case "a166") (case "a167") (case "a168") (case "a169") (case "a170") (case "a171") (case "a172") (case "a173") (case "a174") (case "a175") (case "a176") (case "a177") (case "a178") (case "a179") (case "a180") (case "a181") (case "a182") (case "a183") (case "a184") (case "a185") (case "a186") (case "a187") (case "a188") (case "a189") (case "a190") (case "a191") (case "a192") (case "a193") (case "a194") (case "a195") (case "a196") (case "a197") (case "a198") (case "a199") (case "a200") (case "a201") (case "a202") (case "a203") (case "a204") (case "a205") (case "a206") (case "a207") (case "a208") (case "a209") (case "a210") (case "a211") (case "a212") (case "a213") (case "a214") (case "a215") (case "a216") (case "a217") (case "a218") (case "a219") (case "a220") (case "a221") (case "a222") (case "a223") (case "a224") (case "a225") (case "a226") (case "a227") (case "a228") (case "a229") (case "a230") (case "a231") (case "a232") (case "a233") (case "a234") (case "a235") (case "a236") (case "a237") (case "a238") (case "a239") (case "a240") (case "a241") (case "a242") (case "a243") (case "a244") (case "a245") (case "a246") (case "a247") (case "a248") (case "a249") (case "a250") (case "a251") (case "a252") (case "a253") (case "a254") (case "a255") (case "a256") (case "a257") (case "a258") (case "a259") (case "a260") (case "a261") (case "a262") (case "a263") (case "a264") (case "a265") (case "a266") (case "a267") (case "a268") (case "a269") (case "a270") (case "a271") (case "a272") (case "a273") (case "a274") (case "a275") (case "a276") (case "a277") (case "a278") (case "a279") (case "a280") (case "a281") (case "a282") (case "a283") (case "a284") (case "a285") (case "a286") (case "a287") (case "a288") (case "a289") (case "a290") (case "a291") (case "a292") (case "a293") (case "a294") (case "a295") (case "a296") (case "a297") (case "a298") (case "a299") (case "a300") (case "a301") (case "a302") (case "a303") (case "a304") (case "a305") (case "a306") (case "a307") (case "a308") (case "a309") (case "a310") (case "a311") (case "a312") (case "a313") (case "a314") (case "a315") (case "a316") (case "a317") (case "a318") (case "a319") (case "a320") (case "a321") (case "a322") (case "a323") (case "a324") (case "a325") (case "a326") (case "a327") (case "a328") (case "a329") (case "a330") (case "a331") (case "a332") (case "a333") (case "a334") (case "a335") (case "a336") (case "a337") (case "a338") (case "a339") (case "a340") (case "a341") (case "a342") (case "a343") (case "a344") (case "a345") (case "a346") (case "a347") (case "a348") (case "a349") (case "a350") (case "a351") (case "a352") (case "a353") (case "a354") (case "a355") (case "a356") (case "a357") (case "a358") (case "a359") (case "a360") (case "a361") (case "a362") (case "a363") (case "a364") (case "a365") (case "a366") (case "a367") (case "a368") (case "a369") (case "a370") (case "a371") (case "a372") (case "a373") (case "a374") (case "a375") (case "a376") (case "a377") (case "a378") (case "a379") (case "a380") (case "a381") (case "a382") (case "a383") (case "a384") (case "a385") (case "a386") (case "a387") (case "a388") (case "a389") (case "a390") (case "a391") (case "a392") (case "a393") (case "a394") (case "a395") (case "a396") (case "a397") (case "a398") (case "a399") (case "a400") (case "a401") (case "a402") (case "a403") (case "a404") (case "a405") (case "a406") (case "a407") (case "a408") (case "a409") (case "a410") (case "a411") (case "a412") (case "a413") (case "a414") (case "a415") (case "a416") (case "a417") (case "a418") (case "a419") (case "a420") (case "a421") (case "a422") (case "a423") (case "a424") (case "a425") (case "a426") (case "a427") (case "a428") (case "a429") (case "a430") (case "a431") (case "a432") (case "a433") (case "a434") (case "a435") (case "a436") (case "a437") (case "a438") (case "a439") (case "a440") (case "a441") (case "a442") (case "a443") (case "a444") (case "a445") (case "a446") (case "a447") (case "a448") (case "a449") (case "a450") (case "a451") (case "a452") (case "a453") (case "a454") (case "a455") (case "a456") (case "a457") (case "a458") (case "a459") (case "a460") (case "a461") (case "a462") (case "a463") (case "a464") (case "a465") (case "a466") (case "a467") (case "a468") (case "a469") (case "a470") (case "a471") (case "a472") (case "a473") (case "a474") (case "a475") (case "a476") (case "a477") (case "a478") (case "a479") (case "a480") (case "a481") (case "a482") (case "a483") (case "a484") (case "a485") (case "a486") (case "a487") (case "a488") (case "a489") (case "a490") (case "a491") (case "a492") (case "a493") (case "a494") (case "a495") (case "a496") (case "a497") (case "a498") (case "a499") (case "a500") (case "a501") (case "a502") (case "a503") (case "a504") (case "a505") (case "a506") (case "a507") (case "a508") (case "a509") (case "a510") (case "a511") (case "a512") (case "a513") (case "a514") (case "a515") (case "a516") (case "a517") (case "a518") (case "a519") (case "a520") (case "a521") (case "a522") (case "a523") (case "a524") (case "a525") (case "a526") (case "a527") (case "a528") (case "a529") (case "a530") (case "a531") (case "a532") (case "a533") (case "a534") (case "a535") (case "a536") (case "a537") (case "a538") (case "a539") (case "a540") (case "a541") (case "a542") (case "a543") (case "a544") (case "a545") (case "a546") (case "a547") (case "a548") (case "a549") (case "a550") (case "a551") (case "a552") (case "a553") (case "a554") (case "a555") (case "a556") (case "a557") (case "a558") (case "a559") (case "a560") (case "a561") (case "a562") (case "a563") (case "a564") (case "a565") (case "a566") (case "a567") (case "a568") (case "a569") (case "a570") (case "a571") (case "a572") (case "a573") (case "a574") (case "a575") (case "a576") (case "a577") (case "a578") (case "a579") (case "a580") (case "a581") (case "a582") (case "a583") (case "a584") (case "a585") (case "a586") (case "a587") (case "a588") (case "a589") (case "a590") (case "a591") (case "a592") (case "a593") (case "a594") (case "a595") (case "a596") (case "a597") (case "a598") (case "a599") (case "a600") (case "a601") (case "a602") (case "a603") (case "a604") (case "a605") (case "a606") (case "a607") (case "a608") (case "a609") (case "a610") (case "a611") (case "a612") (case "a613") (case "a614") (case "a615") (case "a616") (case "a617") (case "a618") (case "a619") (case "a620") (case "a621") (case "a622") (case "a623") (case "a624") (case "a625") (case "a626") (case "a627") (case "a628") (case "a629") (case "a630") (case "a631") (case "a632") (case "a633") (case "a634") (case "a635") (case "a636") (case "a637") (case "a638") (case "a639") (case "a640") (case "a641") (case "a642") (case "a643") (case "a644") (case "a645") (case "a646") (case "a647") (case "a648") (case "a649") (case "a650") (case "a651") (case "a652") (case "a653") (case "a654") (case "a655") (case "a656") (case "a657") (case "a658") (case "a659") (case "a660") (case "a661") (case "a662") (case "a663") (case "a664") (case "a665") (case "a666") (case "a667") (case "a668") (case "a669") (case "a670") (case "a671") (case "a672") (case "a673") (case "a674") (case "a675") (case "a676") (case "a677") (case "a678") (case "a679") (case "a680") (case "a681") (case "a682") (case "a683") (case "a684") (case "a685") (case "a686") (case "a687") (case "a688") (case "a689") (case "a690") (case "a691") (case "a692") (case "a693") (case "a694") (case "a695") (case "a696") (case "a697") (case "a698") (case "a699") (case "a700") (case "a701") (case "a702") (case "a703") (case "a704") (case "a705") (case "a706") (case "a707") (case "a708") (case "a709") (case "a710") (case "a711") (case "a712") (case "a713") (case "a714") (case "a715") (case "a716") (case "a717") (case "a718") (case "a719") (case "a720") (case "a721") (case "a722") (case "a723") (case "a724") (case "a725") (case "a726") (case "a727") (case "a728") (case "a729") (case "a730") (case "a731") (case "a732") (case "a733") (case "a734") (case "a735") (case "a736") (case "a737") (case "a738") (case "a739") (case "a740") (case "a741") (case "a742") (case "a743") (case "a744") (case "a745") (case "a746") (case "a747") (case "a748") (case "a749") (case "a750") (case "a751") (case "a752") (case "a753") (case "a754") (case "a755") (case "a756") (case "a757") (case "a758") (case "a759") (case "a760") (case "a761") (case "a762") (case "a763") (case "a764") (case "a765") (case "a766") (case "a767") (case "a768") (case "a769") (case "a770") (case "a771") (case "a772") (case "a773") (case "a774") (case "a775") (case "a776") (case "a777") (case "a778") (case "a779") (case "a780") (case "a781") (case "a782") (case "a783") (case "a784") (case "a785") (case "a786") (case "a787") (case "a788") (case "a789") (case "a790") (case "a791") (case "a792") (case "a793") (case "a794") (case "a795") (case "a796") (case "a797") (case "a798") (case "a799") (case "a800") (case "a801") (case "a802") (case "a803") (case "a804") (case "a805") (case "a806") (case "a807") (case "a808") (case "a809") (case "a810") (case "a811") (case "a812") (case "a813") (case "a814") (case "a815") (case "a816") (case "a817") (case "a818") (case "a819") (case "a820") (case "a821") (case "a822") (case "a823") (case "a824") (case "a825") (case "a826") (case "a827") (case "a828") (case "a829") (case "a830") (case "a831") (case "a832") (case "a833") (case "a834") (case "a835") (case "a836") (case "a837") (case "a838") (case "a839") (case "a840") (case "a841") (case "a842") (case "a843") (case "a844") (case "a845") (case "a846") (case "a847") (case "a848") (case "a849") (case "a850") (case "a851") (case "a852") (case "a853") (case "a854") (case "a855") (case "a856") (case "a857") (case "a858") (case "a859") (case "a860") (case "a861") (case "a862") (case "a863") (case "a864") (case "a865") (case "a866") (case "a867") (case "a868") (case "a869") (case "a870") (case "a871") (case "a872") (case "a873") (case "a874") (case "a875") (case "a876") (case "a877") (case "a878") (case "a879") (case "a880") (case "a881") (case "a882") (case "a883") (case "a884") (case "a885") (case "a886") (case "a887") (case "a888") (case "a889") (case "a890") (case "a891") (case "a892") (case "a893") (case "a894") (case "a895") (case "a896") (case "a897") (case "a898") (case "a899") (case "a900") (case "a901") (case "a902") (case "a903") (case "a904") (case "a905") (case "a906") (case "a907") (case "a908") (case "a909") (case "a910") (case "a911") (case "a912") (case "a913") (case "a914") (case "a915") (case "a916") (case "a917") (case "a918") (case "a919") (case "a920") (case "a921") (case "a922") (case "a923") (case "a924") (case "a925") (case "a926") (case "a927") (case "a928") (case "a929") (case "a930") (case "a931") (case "a932") (case "a933") (case "a934") (case "a935") (case "a936") (case "a937") (case "a938") (case "a939") (case "a940") (case "a941") (case "a942") (case "a943") (case "a944") (case "a945") (case "a946") (case "a947") (case "a948") (case "a949") (case "a950") (case "a951") (case "a952") (case "a953") (case "a954") (case "a955") (case "a956") (case "a957") (case "a958") (case "a959") (case "a960") (case "a961") (case "a962") (case "a963") (case "a964") (case "a965") (case "a966") (case "a967") (case "a968") (case "a969") (case "a970") (case "a971") (case "a972") (case "a973") (case "a974") (case "a975") (case "a976") (case "a977") (case "a978") (case "a979") (case "a980") (case "a981") (case "a982") (case "a983") (case "a984") (case "a985") (case "a986") (case "a987") (case "a988") (case "a989") (case "a990") (case "a991") (case "a992") (case "a993") (case "a994") (case "a995") (case "a996") (case "a997") (case "a998") (case "a999") (case "a1000")))
  (component $provider
    (core module $m
      (func (export "id") (param i32) (result i32) local.get 0)
    )
    (core instance $m (instantiate $m))
    (export $v "value" (type $original))
    (func $id (param "x" $v) (result $v) (canon lift (core func $m "id")))
    (export "id" (func $id))
  )
  (instance $provider (instantiate $provider))
  (component $consumer
    (import "provider" (instance $provider
      (export "value" (type $v (eq $original)))
      (export "id" (func (param "x" $v) (result $v)))
    ))
    (core func $id (canon lower (func $provider "id")))
    (core module $m
      (import "provider" "id" (func $id (param i32) (result i32)))
      (func (export "run") (param i32) (result i32) local.get 0 call $id)
    )
    (core instance $host (export "id" (func $id)))
    (core instance $m (instantiate $m
      (with "provider" (instance
        (export "id" (func $id))
      ))
    ))
    (alias export $provider "value" (type $v))
    (func $run (param "x" $v) (result $v) (canon lift (core func $m "run")))
    (export "run" (func $run))
  )
  (instance $consumer (instantiate $consumer (with "provider" (instance $provider))))
  (export $v "value" (type $provider "value"))
  (export "run" (func $consumer "run") (func (param "x" $v) (result $v)))
)

(assert_return (invoke "run" (variant.const "a0")) (variant.const "a0"))
