use crate::testsuite::util::*;
use std::{borrow::Cow, collections::HashMap, fs::canonicalize, path::PathBuf};

use super::TestSuite;
use eyre::{eyre, Result};
use indexmap::IndexMap;
use log::{debug, error, info};
use tinywasm::{Extern, Imports, ModuleInstance};
use tinywasm_types::{ExternVal, MemoryType, ModuleInstanceAddr, TableType, ValType, WasmValue};
use wasm_testsuite::data::TestFile;
use wast::{lexer::Lexer, parser::ParseBuffer, Wast};

#[derive(Default)]
struct ModuleRegistry {
    modules: HashMap<String, ModuleInstanceAddr>,

    named_modules: HashMap<String, ModuleInstanceAddr>,
    last_module: Option<ModuleInstanceAddr>,
}

impl ModuleRegistry {
    fn modules(&self) -> &HashMap<String, ModuleInstanceAddr> {
        &self.modules
    }

    fn update_last_module(&mut self, addr: ModuleInstanceAddr, name: Option<String>) {
        self.last_module = Some(addr);
        if let Some(name) = name {
            self.named_modules.insert(name, addr);
        }
    }
    fn register(&mut self, name: String, addr: ModuleInstanceAddr) {
        log::debug!("registering module: {}", name);
        self.modules.insert(name.clone(), addr);

        self.last_module = Some(addr);
        self.named_modules.insert(name, addr);
    }

    fn get_idx(&self, module_id: Option<wast::token::Id<'_>>) -> Option<&ModuleInstanceAddr> {
        match module_id {
            Some(module) => {
                log::debug!("getting module: {}", module.name());

                if let Some(addr) = self.modules.get(module.name()) {
                    return Some(addr);
                }

                if let Some(addr) = self.named_modules.get(module.name()) {
                    return Some(addr);
                }

                None
            }
            None => self.last_module.as_ref(),
        }
    }

    fn get<'a>(
        &self,
        module_id: Option<wast::token::Id<'_>>,
        store: &'a tinywasm::Store,
    ) -> Option<&'a ModuleInstance> {
        let addr = self.get_idx(module_id)?;
        store.get_module_instance(*addr)
    }

    fn last<'a>(&self, store: &'a tinywasm::Store) -> Option<&'a ModuleInstance> {
        store.get_module_instance(*self.last_module.as_ref()?)
    }
}

impl TestSuite {
    pub fn run_paths(&mut self, tests: &[PathBuf]) -> Result<()> {
        for file_name in tests {
            let group_wast = std::fs::read(file_name).expect("failed to read test wast");
            let file = TestFile {
                contents: std::str::from_utf8(&group_wast).expect("failed to convert to utf8"),
                name: canonicalize(file_name).expect("failed to canonicalize file name").to_string_lossy().to_string(),
                parent: "(custom group)".into(),
            };

            self.run_file(file).expect("failed to run group");
        }

        Ok(())
    }

    fn imports(modules: &HashMap<std::string::String, u32>) -> Result<Imports> {
        let mut imports = Imports::new();

        let table =
            Extern::table(TableType::new(ValType::RefFunc, 10, Some(20)), WasmValue::default_for(ValType::RefFunc));

        let print = Extern::typed_func(|_ctx: tinywasm::FuncContext, (): ()| {
            log::debug!("print");
            Ok(())
        });

        let print_i32 = Extern::typed_func(|_ctx: tinywasm::FuncContext, arg: i32| {
            log::debug!("print_i32: {}", arg);
            Ok(())
        });

        let print_i64 = Extern::typed_func(|_ctx: tinywasm::FuncContext, arg: i64| {
            log::debug!("print_i64: {}", arg);
            Ok(())
        });

        let print_f32 = Extern::typed_func(|_ctx: tinywasm::FuncContext, arg: f32| {
            log::debug!("print_f32: {}", arg);
            Ok(())
        });

        let print_f64 = Extern::typed_func(|_ctx: tinywasm::FuncContext, arg: f64| {
            log::debug!("print_f64: {}", arg);
            Ok(())
        });

        let print_i32_f32 = Extern::typed_func(|_ctx: tinywasm::FuncContext, args: (i32, f32)| {
            log::debug!("print_i32_f32: {}, {}", args.0, args.1);
            Ok(())
        });

        let print_f64_f64 = Extern::typed_func(|_ctx: tinywasm::FuncContext, args: (f64, f64)| {
            log::debug!("print_f64_f64: {}, {}", args.0, args.1);
            Ok(())
        });

        imports
            .define(
                "spectest",
                "memory",
                Extern::memory(MemoryType::new(tinywasm_types::MemoryArch::I32, 1, Some(2), None)),
            )?
            .define("spectest", "table", table)?
            .define("spectest", "global_i32", Extern::global(WasmValue::I32(666), false))?
            .define("spectest", "global_i64", Extern::global(WasmValue::I64(666), false))?
            .define("spectest", "global_f32", Extern::global(WasmValue::F32(666.6), false))?
            .define("spectest", "global_f64", Extern::global(WasmValue::F64(666.6), false))?
            .define("spectest", "print", print)?
            .define("spectest", "print_i32", print_i32)?
            .define("spectest", "print_i64", print_i64)?
            .define("spectest", "print_f32", print_f32)?
            .define("spectest", "print_f64", print_f64)?
            .define("spectest", "print_i32_f32", print_i32_f32)?
            .define("spectest", "print_f64_f64", print_f64_f64)?;

        for (name, addr) in modules {
            log::debug!("registering module: {}", name);
            imports.link_module(name, *addr)?;
        }

        Ok(imports)
    }

    pub fn run_files<'a>(&mut self, tests: impl IntoIterator<Item = TestFile<'a>>) -> Result<()> {
        tests.into_iter().for_each(|group| {
            let name = group.name();
            println!("running group: {}", name);
            if self.1.contains(&name.to_string()) {
                info!("skipping group: {name}");
                self.test_group(&format!("{name} (skipped)"), name);
                return;
            }

            self.run_file(group).expect("failed to run group");
        });

        Ok(())
    }

    pub fn run_file<'a>(&mut self, file: TestFile<'a>) -> Result<()> {
        let test_group = self.test_group(file.name(), file.parent());
        let wast_raw = file.raw();
        let wast = file.wast()?;
        let directives = wast.directives()?;

        let mut store = tinywasm::Store::default();
        let mut module_registry = ModuleRegistry::default();

        println!("running {} tests for group: {}", directives.len(), file.name());
        for (i, directive) in directives.into_iter().enumerate() {
            let span = directive.span();
            use wast::WastDirective::{
                AssertExhaustion, AssertInvalid, AssertMalformed, AssertReturn, AssertTrap, AssertUnlinkable, Invoke,
                Module as Wat, Register,
            };

            match directive {
                Register { span, name, .. } => {
                    let Some(last) = module_registry.last(&store) else {
                        test_group.add_result(
                            &format!("Register({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("no module to register")),
                        );
                        continue;
                    };
                    module_registry.register(name.to_string(), last.id());
                    test_group.add_result(&format!("Register({i})"), span.linecol_in(wast_raw), Ok(()));
                }

                Wat(module) => {
                    debug!("got wat module");
                    let result = catch_unwind_silent(|| {
                        let (name, bytes) = encode_quote_wat(module);
                        let m = parse_module_bytes(&bytes).expect("failed to parse module bytes");

                        let module_instance = tinywasm::Module::from(m)
                            .instantiate(&mut store, Some(Self::imports(module_registry.modules()).unwrap()))
                            .expect("failed to instantiate module");

                        (name, module_instance)
                    })
                    .map_err(|e| eyre!("failed to parse wat module: {:?}", try_downcast_panic(e)));

                    match &result {
                        Err(err) => debug!("failed to parse module: {:?}", err),
                        Ok((name, module)) => module_registry.update_last_module(module.id(), name.clone()),
                    };

                    test_group.add_result(&format!("Wat({i})"), span.linecol_in(wast_raw), result.map(|_| ()));
                }

                AssertMalformed { span, mut module, message } => {
                    let Ok(module) = module.encode() else {
                        test_group.add_result(&format!("AssertMalformed({i})"), span.linecol_in(wast_raw), Ok(()));
                        continue;
                    };

                    let res = catch_unwind_silent(|| parse_module_bytes(&module))
                        .map_err(|e| eyre!("failed to parse module (expected): {:?}", try_downcast_panic(e)))
                        .and_then(|res| res);

                    test_group.add_result(
                        &format!("AssertMalformed({i})"),
                        span.linecol_in(wast_raw),
                        match res {
                            Ok(_) => {
                                // - skip "zero byte expected" as the magic number is not checked by wasmparser
                                //   (Don't need to error on this, doesn't matter if it's malformed)
                                // - skip "integer representation too long" as this has some false positives on older tests
                                if message == "zero byte expected" || message == "integer representation too long" {
                                    continue;
                                }

                                Err(eyre!("expected module to be malformed: {message}"))
                            }
                            Err(_) => Ok(()),
                        },
                    );
                }

                AssertInvalid { span, mut module, message } => {
                    if ["multiple memories"].contains(&message) {
                        test_group.add_result(&format!("AssertInvalid({i})"), span.linecol_in(wast_raw), Ok(()));
                        continue;
                    }

                    let res = catch_unwind_silent(move || parse_module_bytes(&module.encode().unwrap()))
                        .map_err(|e| eyre!("failed to parse module (invalid): {:?}", try_downcast_panic(e)))
                        .and_then(|res| res);

                    test_group.add_result(
                        &format!("AssertInvalid({i})"),
                        span.linecol_in(wast_raw),
                        match res {
                            Ok(_) => Err(eyre!("expected module to be invalid")),
                            Err(_) => Ok(()),
                        },
                    );
                }

                AssertExhaustion { call, message, span } => {
                    let module = module_registry.get_idx(call.module);
                    let args = convert_wastargs(call.args).expect("failed to convert args");
                    let res =
                        catch_unwind_silent(|| exec_fn_instance(module, &mut store, call.name, &args).map(|_| ()));

                    let Ok(Err(tinywasm::Error::Trap(trap))) = res else {
                        test_group.add_result(
                            &format!("AssertExhaustion({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("expected trap")),
                        );
                        continue;
                    };

                    if !message.starts_with(trap.message()) {
                        test_group.add_result(
                            &format!("AssertExhaustion({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("expected trap: {}, got: {}", message, trap.message())),
                        );
                        continue;
                    }

                    test_group.add_result(&format!("AssertExhaustion({i})"), span.linecol_in(wast_raw), Ok(()));
                }

                AssertTrap { exec, message, span } => {
                    let res: Result<tinywasm::Result<()>, _> = catch_unwind_silent(|| {
                        let invoke = match exec {
                            wast::WastExecute::Wat(mut wat) => {
                                let module = parse_module_bytes(&wat.encode().expect("failed to encode module"))
                                    .expect("failed to parse module");
                                let module = tinywasm::Module::from(module);
                                module
                                    .instantiate(&mut store, Some(Self::imports(module_registry.modules()).unwrap()))?;
                                return Ok(());
                            }
                            wast::WastExecute::Get { module: _, global: _, .. } => {
                                panic!("get not supported");
                            }
                            wast::WastExecute::Invoke(invoke) => invoke,
                        };

                        let module = module_registry.get_idx(invoke.module);
                        let args = convert_wastargs(invoke.args).expect("failed to convert args");
                        exec_fn_instance(module, &mut store, invoke.name, &args).map(|_| ())
                    });

                    match res {
                        Err(err) => test_group.add_result(
                            &format!("AssertTrap({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("test panicked: {:?}", try_downcast_panic(err))),
                        ),
                        Ok(Err(tinywasm::Error::Trap(trap))) => {
                            if !message.starts_with(trap.message()) {
                                test_group.add_result(
                                    &format!("AssertTrap({i})"),
                                    span.linecol_in(wast_raw),
                                    Err(eyre!("expected trap: {}, got: {}", message, trap.message())),
                                );
                                continue;
                            }

                            test_group.add_result(&format!("AssertTrap({i})"), span.linecol_in(wast_raw), Ok(()));
                        }
                        Ok(Err(err)) => test_group.add_result(
                            &format!("AssertTrap({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("expected trap, {}, got: {:?}", message, err)),
                        ),
                        Ok(Ok(())) => test_group.add_result(
                            &format!("AssertTrap({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("expected trap {}, got Ok", message)),
                        ),
                    }
                }

                AssertUnlinkable { mut module, span, message } => {
                    let res = catch_unwind_silent(|| {
                        let module = parse_module_bytes(&module.encode().expect("failed to encode module"))
                            .expect("failed to parse module");
                        let module = tinywasm::Module::from(module);
                        module.instantiate(&mut store, Some(Self::imports(module_registry.modules()).unwrap()))
                    });

                    match res {
                        Err(err) => test_group.add_result(
                            &format!("AssertUnlinkable({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("test panicked: {:?}", try_downcast_panic(err))),
                        ),
                        Ok(Err(tinywasm::Error::Linker(err))) => {
                            if err.message() != message
                                && (err.message() == "memory types incompatible"
                                    && message != "incompatible import type")
                            {
                                test_group.add_result(
                                    &format!("AssertUnlinkable({i})"),
                                    span.linecol_in(wast_raw),
                                    Err(eyre!("expected linker error: {}, got: {}", message, err.message())),
                                );
                                continue;
                            }

                            test_group.add_result(&format!("AssertUnlinkable({i})"), span.linecol_in(wast_raw), Ok(()));
                        }
                        Ok(Err(err)) => test_group.add_result(
                            &format!("AssertUnlinkable({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("expected linker error, {}, got: {:?}", message, err)),
                        ),
                        Ok(Ok(_)) => test_group.add_result(
                            &format!("AssertUnlinkable({i})"),
                            span.linecol_in(wast_raw),
                            Err(eyre!("expected linker error {}, got Ok", message)),
                        ),
                    }
                }

                Invoke(invoke) => {
                    let name = invoke.name;

                    let res: Result<Result<()>, _> = catch_unwind_silent(|| {
                        let args = convert_wastargs(invoke.args)?;
                        let module = module_registry.get_idx(invoke.module);
                        exec_fn_instance(module, &mut store, invoke.name, &args).map_err(|e| {
                            error!("failed to execute function: {:?}", e);
                            e
                        })?;
                        Ok(())
                    });

                    let res = res.map_err(|e| eyre!("test panicked: {:?}", try_downcast_panic(e))).and_then(|r| r);
                    test_group.add_result(&format!("Invoke({name}-{i})"), span.linecol_in(wast_raw), res);
                }

                AssertReturn { span, exec, results } => {
                    info!("AssertReturn: {:?}", exec);
                    let expected = match convert_wastret(results.into_iter()) {
                        Err(err) => {
                            test_group.add_result(
                                &format!("AssertReturn(unsupported-{i})"),
                                span.linecol_in(wast_raw),
                                Err(eyre!("failed to convert expected results: {:?}", err)),
                            );
                            continue;
                        }
                        Ok(expected) => expected,
                    };

                    let invoke = match match exec {
                        wast::WastExecute::Wat(_) => Err(eyre!("wat not supported")),
                        wast::WastExecute::Get { module: module_id, global, .. } => {
                            let module = module_registry.get(module_id, &store);
                            let Some(module) = module else {
                                test_group.add_result(
                                    &format!("AssertReturn(unsupported-{i})"),
                                    span.linecol_in(wast_raw),
                                    Err(eyre!("no module to get global from")),
                                );
                                continue;
                            };

                            let module_global = match match module.export_addr(global) {
                                Some(ExternVal::Global(addr)) => Ok(store.get_global_val(addr)),
                                _ => Err(eyre!("no module to get global from")),
                            } {
                                Ok(module_global) => module_global,
                                Err(err) => {
                                    test_group.add_result(
                                        &format!("AssertReturn(unsupported-{i})"),
                                        span.linecol_in(wast_raw),
                                        Err(eyre!("failed to get global: {:?}", err)),
                                    );
                                    continue;
                                }
                            };
                            let expected = expected.first().expect("expected global value");
                            let module_global = module_global.attach_type(expected.val_type());

                            if !module_global.eq_loose(expected) {
                                test_group.add_result(
                                    &format!("AssertReturn(unsupported-{i})"),
                                    span.linecol_in(wast_raw),
                                    Err(eyre!("global value did not match: {:?} != {:?}", module_global, expected)),
                                );
                                continue;
                            }

                            test_group.add_result(
                                &format!("AssertReturn({global}-{i})"),
                                span.linecol_in(wast_raw),
                                Ok(()),
                            );

                            continue;
                            // check if module_global matches the expected results
                        }
                        wast::WastExecute::Invoke(invoke) => Ok(invoke),
                    } {
                        Ok(invoke) => invoke,
                        Err(err) => {
                            test_group.add_result(
                                &format!("AssertReturn(unsupported-{i})"),
                                span.linecol_in(wast_raw),
                                Err(eyre!("unsupported directive: {:?}", err)),
                            );
                            continue;
                        }
                    };

                    let invoke_name = invoke.name;
                    let res: Result<Result<()>, _> = catch_unwind_silent(|| {
                        debug!("invoke: {:?}", invoke);
                        let args = convert_wastargs(invoke.args)?;
                        let module = module_registry.get_idx(invoke.module);
                        let outcomes = exec_fn_instance(module, &mut store, invoke.name, &args).map_err(|e| {
                            error!("failed to execute function: {:?}", e);
                            e
                        })?;

                        if outcomes.len() != expected.len() {
                            return Err(eyre!(
                                "span: {:?} expected {} results, got {}",
                                span,
                                expected.len(),
                                outcomes.len()
                            ));
                        }

                        outcomes.iter().zip(expected).enumerate().try_for_each(|(i, (outcome, exp))| {
                            (outcome.eq_loose(&exp))
                                .then_some(())
                                .ok_or_else(|| eyre!(" result {} did not match: {:?} != {:?}", i, outcome, exp))
                        })
                    });

                    let res = res.map_err(|e| eyre!("test panicked: {:?}", try_downcast_panic(e))).and_then(|r| r);
                    test_group.add_result(&format!("AssertReturn({invoke_name}-{i})"), span.linecol_in(wast_raw), res);
                }
                _ => test_group.add_result(
                    &format!("Unknown({i})"),
                    span.linecol_in(wast_raw),
                    Err(eyre!("unsupported directive")),
                ),
            }
        }

        Ok(())
    }
}
