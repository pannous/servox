;; Simple WASM GC struct test
(module
  ;; Simple struct with one i32 field
  (type $box (struct (field $val (mut i32))))

  ;; Create a box with a value
  (func $makeBox (export "makeBox") (param $v i32) (result (ref $box))
    local.get $v
    struct.new $box
  )

  ;; Get the value from the box
  (func $getValue (export "getValue") (param $b (ref $box)) (result i32)
    local.get $b
    struct.get $box 0
  )

  ;; Set the value in the box
  (func $setValue (export "setValue") (param $b (ref $box)) (param $v i32)
    local.get $b
    local.get $v
    struct.set $box 0
  )
)
