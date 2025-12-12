# Servo Fork
Fork of official [servo](https://github.com/servo/servo) browser (engine) with the following modifications:

✅ <script type="text/wast">  
✅ <script type="text/typescript">  
✅ <script type="application/wasm" src="…">  

E.g. 
```
  <script type="text/wast">
(module
  (func $getValue (result i32)
    i32.const 123)
  (export "getValue" (func $getValue)))
  </script> 

    <script type="text/typescript">
        const greeting: string = "Hello from TypeScript!";
        const version: number = getValue()
        console.log(`${greeting} (v${version})`);
    </script>
```

💡exports are immediately available to JavaScript,  even gc objects!

```
<script type="text/wast">
(module
  (type $Box (struct (field $val (mut i32))))
  (global $box (export "box") (ref $Box) (struct.new $Box (i32.const 42)))
)
  </script>
<script type="text/typescript">
  console.log(box.val);
</script>
```

## Installation

Currently this is more for experimental development purposes but if you want to play with it without re-compiling you can

```bash
brew tap pannous/servox
brew install servox
```
