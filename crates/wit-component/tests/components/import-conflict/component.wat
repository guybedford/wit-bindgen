(component
  (type (;0;) (func (param "x" u64) (param "y" string)))
  (type (;1;) 
    (instance
      (alias outer 1 0 (type (;0;)))
      (export "a" (func (type 0)))
    )
  )
  (type (;2;) (list u8))
  (type (;3;) (func (param "x" 2) (result 2)))
  (type (;4;) 
    (instance
      (alias outer 1 3 (type (;0;)))
      (export "baz" (func (type 0)))
    )
  )
  (type (;5;) (func))
  (type (;6;) 
    (instance
      (alias outer 1 5 (type (;0;)))
      (export "a" (func (type 0)))
    )
  )
  (import "bar" (instance (;0;) (type 1)))
  (import "baz" (instance (;1;) (type 4)))
  (import "foo" (instance (;2;) (type 6)))
  (core module (;0;)
    (type (;0;) (func))
    (type (;1;) (func (param i64 i32 i32)))
    (type (;2;) (func (param i32 i32 i32)))
    (type (;3;) (func (param i32 i32 i32 i32) (result i32)))
    (import "foo" "a" (func (;0;) (type 0)))
    (import "bar" "a" (func (;1;) (type 1)))
    (import "baz" "baz" (func (;2;) (type 2)))
    (func (;3;) (type 3) (param i32 i32 i32 i32) (result i32)
      unreachable
    )
    (memory (;0;) 1)
    (export "memory" (memory 0))
    (export "cabi_realloc" (func 3))
  )
  (core module (;1;)
    (type (;0;) (func (param i64 i32 i32)))
    (type (;1;) (func (param i32 i32 i32)))
    (func (;0;) (type 0) (param i64 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      i32.const 0
      call_indirect (type 0)
    )
    (func (;1;) (type 1) (param i32 i32 i32)
      local.get 0
      local.get 1
      local.get 2
      i32.const 1
      call_indirect (type 1)
    )
    (table (;0;) 2 2 funcref)
    (export "0" (func 0))
    (export "1" (func 1))
    (export "$imports" (table 0))
  )
  (core module (;2;)
    (type (;0;) (func (param i64 i32 i32)))
    (type (;1;) (func (param i32 i32 i32)))
    (import "" "0" (func (;0;) (type 0)))
    (import "" "1" (func (;1;) (type 1)))
    (import "" "$imports" (table (;0;) 2 2 funcref))
    (elem (;0;) (i32.const 0) func 0 1)
  )
  (core instance (;0;) (instantiate 1))
  (alias export 2 "a" (func (;0;)))
  (core func (;0;) (canon lower (func 0)))
  (core instance (;1;) 
    (export "a" (func 0))
  )
  (alias core export 0 "0" (core func (;1;)))
  (core instance (;2;) 
    (export "a" (func 1))
  )
  (alias core export 0 "1" (core func (;2;)))
  (core instance (;3;) 
    (export "baz" (func 2))
  )
  (core instance (;4;) (instantiate 0
      (with "foo" (instance 1))
      (with "bar" (instance 2))
      (with "baz" (instance 3))
    )
  )
  (alias core export 4 "memory" (core memory (;0;)))
  (alias core export 4 "cabi_realloc" (core func (;3;)))
  (alias core export 0 "$imports" (core table (;0;)))
  (alias export 0 "a" (func (;1;)))
  (core func (;4;) (canon lower (func 1) (memory 0) string-encoding=utf8))
  (alias export 1 "baz" (func (;2;)))
  (core func (;5;) (canon lower (func 2) (memory 0) (realloc 3)))
  (core instance (;5;) 
    (export "$imports" (table 0))
    (export "0" (func 4))
    (export "1" (func 5))
  )
  (core instance (;6;) (instantiate 2
      (with "" (instance 5))
    )
  )
)