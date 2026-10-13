(component
  (import "synt:extension/types@0.1.0" (instance $types
    (type $risk-def (enum "read" "write" "privileged"))
    (export "risk" (type $risk (eq $risk-def)))
    (type $action-def (record
      (field "id" string)
      (field "title" string)
      (field "risk" $risk)
      (field "takes-target" bool)
      (field "params-schema" string)))
    (export "action" (type (eq $action-def)))
    (type $error-def (record (field "code" string) (field "message" string)))
    (export "error" (type (eq $error-def)))
    (type $level-def (enum "trace" "debug" "info" "warn" "error"))
    (export "level" (type (eq $level-def)))))
  (alias export $types "action" (type $action))
  (alias export $types "error" (type $error))
  (alias export $types "level" (type $level))

  (import "call" (func $call
    (param "action" string)
    (param "target" (option string))
    (param "params" string)
    (result (result string (error $error)))))
  (import "storage-get" (func $storage-get (param "key" string) (result (option (list u8)))))
  (import "storage-set" (func $storage-set
    (param "key" string)
    (param "value" (list u8))
    (result (result (error $error)))))
  (import "log" (func $log (param "level" $level) (param "message" string)))

  (core module $Memory
    (memory (export "memory") 1)
    (global $heap (mut i32) (i32.const 32768))
    (func (export "realloc") (param i32 i32 i32 i32) (result i32)
      (local $start i32)
      (local.set $start
        (i32.and
          (i32.add (global.get $heap) (i32.sub (local.get 2) (i32.const 1)))
          (i32.sub (i32.const 0) (local.get 2))))
      (global.set $heap (i32.add (local.get $start) (local.get 3)))
      (if (i32.gt_u (global.get $heap) (i32.mul (memory.size) (i32.const 65536)))
        (then
          (if (i32.eq
                (memory.grow
                  (i32.add
                    (i32.shr_u
                      (i32.sub (global.get $heap) (i32.mul (memory.size) (i32.const 65536)))
                      (i32.const 16))
                    (i32.const 1)))
                (i32.const -1))
            (then unreachable))))
      (local.get $start)))
  (core instance $memory (instantiate $Memory))
  (alias core export $memory "memory" (core memory $mem))
  (alias core export $memory "realloc" (core func $realloc))

  (core func $call-lowered (canon lower (func $call) (memory $mem) (realloc $realloc)))
  (core func $storage-get-lowered (canon lower (func $storage-get) (memory $mem) (realloc $realloc)))
  (core func $storage-set-lowered (canon lower (func $storage-set) (memory $mem) (realloc $realloc)))
  (core func $log-lowered (canon lower (func $log) (memory $mem)))

  (core module $Main
    (import "env" "memory" (memory 1))
    (import "host" "call" (func $call (param i32 i32 i32 i32 i32 i32 i32 i32)))
    (import "host" "storage-get" (func $storage-get (param i32 i32 i32)))
    (import "host" "storage-set" (func $storage-set (param i32 i32 i32 i32 i32)))
    (import "host" "log" (func $log (param i32 i32 i32)))
    (data (i32.const 64) "\7b\22\74\79\70\65\22\3a\22\6f\62\6a\65\63\74\22\7d\00\00\00\74\2e\65\63\68\6f\00\00\45\63\68\6f\20\74\68\65\20\70\61\72\61\6d\73\00\74\2e\73\70\69\6e\00\00\53\70\69\6e\20\66\6f\72\65\76\65\72\74\2e\63\61\6c\6c\00\00\52\75\6e\20\61\20\63\6f\6d\6d\61\6e\64\20\74\68\72\6f\75\67\68\20\73\73\68\2e\65\78\65\63\00\00\74\2e\73\74\6f\72\65\00\4b\65\65\70\20\74\68\65\20\70\61\72\61\6d\73\20\69\6e\20\73\74\6f\72\61\67\65\20\61\6e\64\20\72\65\61\64\20\74\68\65\6d\20\62\61\63\6b\00\00\00\74\2e\66\61\69\6c\00\00\46\61\69\6c\20\77\69\74\68\20\61\6e\20\65\72\72\6f\72\00\00\74\2e\67\72\6f\77\00\00\41\73\6b\20\66\6f\72\20\6d\6f\72\65\20\6d\65\6d\6f\72\79\20\74\68\61\6e\20\61\6c\6c\6f\77\65\64\73\73\68\2e\65\78\65\63\6b\00\00\00\62\6f\6f\6d\69\74\20\62\72\6f\6b\65\6f\6f\6d\00\6e\6f\20\6d\65\6d\6f\72\79\00\00\00\63\61\6c\6c\69\6e\67\20\73\73\68\2e\65\78\65\63\65\6d\70\74\79")
    (data (i32.const 2048) "\54\00\00\00\06\00\00\00\5c\00\00\00\0f\00\00\00\00\00\00\00\40\00\00\00\11\00\00\00\6c\00\00\00\06\00\00\00\74\00\00\00\0c\00\00\00\00\00\00\00\40\00\00\00\11\00\00\00\80\00\00\00\06\00\00\00\88\00\00\00\1e\00\00\00\01\01\00\00\40\00\00\00\11\00\00\00\a8\00\00\00\07\00\00\00\b0\00\00\00\2d\00\00\00\01\00\00\00\40\00\00\00\11\00\00\00\e0\00\00\00\06\00\00\00\e8\00\00\00\12\00\00\00\00\00\00\00\40\00\00\00\11\00\00\00\fc\00\00\00\06\00\00\00\04\01\00\00\20\00\00\00\00\00\00\00\40\00\00\00\11\00\00\00")
    (data (i32.const 1024) "\00\08\00\00\06\00\00\00")
    (func $ok (param i32 i32) (result i32)
      (i32.store8 (i32.const 1100) (i32.const 0))
      (i32.store (i32.const 1104) (local.get 0))
      (i32.store (i32.const 1108) (local.get 1))
      (i32.const 1100))
    (func $err (param i32 i32 i32 i32) (result i32)
      (i32.store8 (i32.const 1100) (i32.const 1))
      (i32.store (i32.const 1104) (local.get 0))
      (i32.store (i32.const 1108) (local.get 1))
      (i32.store (i32.const 1112) (local.get 2))
      (i32.store (i32.const 1116) (local.get 3))
      (i32.const 1100))
    (func $store (param i32 i32) (result i32)
      (call $storage-set (i32.const 300) (i32.const 1) (local.get 0) (local.get 1) (i32.const 1300))
      (if (i32.ne (i32.load8_u (i32.const 1300)) (i32.const 0))
        (then
          (return
            (call $err
              (i32.load (i32.const 1304))
              (i32.load (i32.const 1308))
              (i32.load (i32.const 1312))
              (i32.load (i32.const 1316))))))
      (call $storage-get (i32.const 300) (i32.const 1) (i32.const 1200))
      (if (i32.eqz (i32.load8_u (i32.const 1200)))
        (then (return (call $err (i32.const 348) (i32.const 5) (i32.const 348) (i32.const 5)))))
      (call $ok (i32.load (i32.const 1204)) (i32.load (i32.const 1208))))
    (func (export "describe") (result i32)
      (i32.const 1024))
    (func (export "run") (param i32 i32 i32 i32 i32 i32 i32) (result i32)
      (local $letter i32)
      (local.set $letter (i32.load8_u offset=2 (local.get 0)))
      (if (i32.eq (local.get $letter) (i32.const 101))
        (then (return (call $ok (local.get 5) (local.get 6)))))
      (if (i32.and (i32.eq (local.get $letter) (i32.const 115)) (i32.eq (local.get 1) (i32.const 6)))
        (then (loop $forever (br $forever))))
      (if (i32.and (i32.eq (local.get $letter) (i32.const 115)) (i32.eq (local.get 1) (i32.const 7)))
        (then (return (call $store (local.get 5) (local.get 6)))))
      (if (i32.eq (local.get $letter) (i32.const 99))
        (then
          (call $log (i32.const 2) (i32.const 332) (i32.const 16))
          (call $call
            (i32.const 292) (i32.const 8)
            (local.get 2) (local.get 3) (local.get 4)
            (local.get 5) (local.get 6)
            (i32.const 1100))
          (return (i32.const 1100))))
      (if (i32.eq (local.get $letter) (i32.const 102))
        (then (return (call $err (i32.const 304) (i32.const 4) (i32.const 308) (i32.const 8)))))
      (if (i32.eq (local.get $letter) (i32.const 103))
        (then
          (if (i32.eq (memory.grow (i32.const 2000)) (i32.const -1))
            (then (return (call $err (i32.const 316) (i32.const 3) (i32.const 320) (i32.const 9)))))
          (return (call $ok (i32.const 348) (i32.const 5)))))
      unreachable))
  (core instance $main (instantiate $Main
    (with "env" (instance (export "memory" (memory $mem))))
    (with "host" (instance
      (export "call" (func $call-lowered))
      (export "storage-get" (func $storage-get-lowered))
      (export "storage-set" (func $storage-set-lowered))
      (export "log" (func $log-lowered))))))

  (func $describe (result (list $action))
    (canon lift (core func $main "describe") (memory $mem)))
  (func $run
    (param "action" string)
    (param "target" (option string))
    (param "params" string)
    (result (result string (error $error)))
    (canon lift (core func $main "run") (memory $mem) (realloc $realloc)))
  (export "describe" (func $describe))
  (export "run" (func $run)))
