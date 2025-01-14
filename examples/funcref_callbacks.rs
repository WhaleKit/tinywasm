use eyre::Result;
use tinywasm::{types::FuncRef, Extern, FuncContext, Imports, Module, Store};
use wat;

fn main() -> Result<()> {
    by_func_ref_passed()?;
    by_func_ref_returned()?;
    Ok(())
}

/// example of passing wasm functions (as funcref) to imported host function
/// and imported host function calling them
fn by_func_ref_passed() -> Result<()> {
    // a module with:
    // imported function "host.call_this" that accepts a callback
    // exported wasm function "tell_host_to_call" that calls "host.call_this" with wasm functions $add and $sub
    // wasm functions $add and $sub and imported function $mul used as callbacks
    // (just to show that imported functions can be referenced too)
    // exported wasm function "call_binop_by_ref" is a proxy used by host to call func-references of type (i32, i32)->i32
    const WASM: &str = r#"
    (module
        (import "host" "call_this" (func $host_callback_caller (param funcref)))
        (import "host" "mul" (func $host_mul (param $x i32) (param $y i32) (result i32)))
      
        (func $tell_host_to_call (export "tell_host_to_call")
          	(call $host_callback_caller (ref.func $add))
          	(call $host_callback_caller (ref.func $sub))
          	(call $host_callback_caller (ref.func $host_mul))
        )
      
        (type $binop (func (param i32 i32) (result i32)))
        
        (table 3 funcref)
		(elem (i32.const 0) $add $sub $host_mul) ;; function can only be taken reference of if it's added to a table
        (func $add (param $x i32) (param $y i32) (result i32)
            local.get $x
            local.get $y
            i32.add
        )
        (func $sub (param $x i32) (param $y i32) (result i32)
            local.get $x
            local.get $y
            i32.sub
        )

        (table $callback_register 1 funcref)
        (func (export "call_binop_by_ref") (param funcref i32 i32) (result i32)
          	(table.set $callback_register (i32.const 0) (local.get 0))
            (call_indirect $callback_register (type $binop) (local.get 1)(local.get 2)(i32.const 0))
        )
    )
    "#;

    let wasm = wat::parse_str(WASM).expect("failed to parse wat");
    let module = Module::parse_bytes(&wasm)?;
    let mut store = Store::default();
    let mut imports = Imports::new();
    // import host function that takes callbacks and calls them
    imports.define(
        "host",
        "call_this",
        Extern::typed_func(|mut ctx: FuncContext<'_>, fn_ref: FuncRef| -> tinywasm::Result<()> {
            let proxy_caller =
                ctx.module().exported_func::<(FuncRef, i32, i32), i32>(ctx.store(), "call_binop_by_ref")?;
            // call callback we got as argument using call_binop_by_ref
            let res = proxy_caller.call(ctx.store_mut(), (fn_ref, 5, 3))?;
            println!("(funcref {fn_ref:?})(5,3) results in {res}");

            Ok(())
        }),
    )?;
    // import host.mul function (one of the functions whose references are taken)
    imports.define(
        "host",
        "mul",
        Extern::typed_func(|_, args: (i32, i32)| -> tinywasm::Result<i32> { Ok(args.0 * args.1) }),
    )?;
    let instance = module.instantiate(&mut store, Some(imports))?;
    let caller = instance.exported_func::<(), ()>(&mut store, "tell_host_to_call")?;
    // call the tell_host_to_call
    caller.call(&mut store, ())?;
    // interesting detail is that neither $add $sub $mul were exported,
    // but with a little help from proxy "call_binop_by_ref" references to them host was able to call them
    Ok(())
}

/// example of returning wasm function as callback to host function
/// and host function calling it
fn by_func_ref_returned() -> Result<()> {
    // a module with:
    // an exported function "what_should_host_call" that returns 3 funcrefs
    // wasm functions $add and $sub and imported function $mul used as callbacks
    // (just to show that imported functions can be referenced too)
    // exported wasm function "call_binop_by_ref" is a proxy used by host to call func-references of type (i32, i32)->i32
    const WASM: &str = r#"
    (module
        (import "host" "mul" (func $host_mul (param $x i32) (param $y i32) (result i32)))
        (type $binop (func (param i32 i32) (result i32)))
        (table 3 funcref)
		(elem (i32.const 0) $add $sub $host_mul)
        (func $add (param $x i32) (param $y i32) (result i32)
            local.get $x
            local.get $y
            i32.add
        )
        (func $sub (param $x i32) (param $y i32) (result i32)
            local.get $x
            local.get $y
            i32.sub
        )
        (func $ref_to_funcs (export "what_should_host_call")  (result funcref funcref funcref)
      	    (ref.func $add)
          	(ref.func $sub)
            (ref.func $host_mul)
        )

        (table $callback_register 1 funcref)
        (func $call (export "call_binop_by_ref") (param funcref i32 i32) (result i32)
          	(table.set $callback_register (i32.const 0) (local.get 0))
            (call_indirect $callback_register (type $binop) (local.get 1)(local.get 2)(i32.const 0))
        )
    )
    "#;

    let wasm = wat::parse_str(WASM).expect("failed to parse wat");
    let module = Module::parse_bytes(&wasm)?;
    let mut store = Store::default();
    let mut imports = Imports::new();
    // import host.mul function (one of the possible operations)
    imports.define(
        "host",
        "mul",
        Extern::typed_func(|_, args: (i32, i32)| -> tinywasm::Result<i32> { Ok(args.0 * args.1) }),
    )?;

    let instance = module.instantiate(&mut store, Some(imports))?;
    {
        // ask module what should we call
        let funcrefs = {
            let address_getter =
                instance.exported_func::<(), (FuncRef, FuncRef, FuncRef)>(&mut store, "what_should_host_call")?;
            address_getter.call(&mut store, ())?
        };
        let proxy_caller = instance.exported_func::<(FuncRef, i32, i32), i32>(&mut store, "call_binop_by_ref")?;
        for (idx, func_ref) in [funcrefs.0, funcrefs.1, funcrefs.2].iter().enumerate() {
            // call those funcrefs via "call_binop_by_ref"
            let res = proxy_caller.call(&mut store, (*func_ref, 5, 3))?;
            println!("at idx: {idx} funcref {func_ref:?}(5,3) results in {res}");
        }
    }
    Ok(())
}
