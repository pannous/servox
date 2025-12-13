(module
  (type $string (array (mut i8)))
  (type $Box (struct (field $val (mut (ref $string)))))
  (func $Box.new (export "newBox") (param (ref $string)) (result (ref $Box))
    local.get 0
    struct.new $Box
  )
    (data $hello "hello")
  (global $box (export "box") (ref $Box) (array.new_data $string $hello (i32.const 5)) )
  ;;(global $box (export "box") (ref $Box) (array.new_canon_data $string $hello (i32.const 5)) )
)