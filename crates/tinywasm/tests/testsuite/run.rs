use crate::testsuite::util::*;
use std::borrow::Cow;

use super::TestSuite;
use eyre::{eyre, Result};
use log::{debug, error};
use tinywasm_types::TinyWasmModule;
use wast::{lexer::Lexer, parser::ParseBuffer, Wast};

impl TestSuite {
    pub fn run_paths(&mut self, tests: &[&str]) -> Result<()> {
        tests.iter().for_each(|group| {
            let group_wast = std::fs::read(group).expect("failed to read test wast");
            let group_wast = Cow::Owned(group_wast);
            debug!("running group: {}", group);
            self.run_group(group, group_wast).expect("failed to run group");
        });

        Ok(())
    }

    pub fn run_spec_group(&mut self, tests: &[&str]) -> Result<()> {
        tests.iter().for_each(|group| {
            let group_wast = wasm_testsuite::get_test_wast(group).expect("failed to get test wast");
            self.run_group(group, group_wast).expect("failed to run group");
        });

        Ok(())
    }

    pub fn run_group(&mut self, group_name: &str, group_wast: Cow<'_, [u8]>) -> Result<()> {
        let test_group = self.test_group(group_name);
        let wast = std::str::from_utf8(&group_wast).expect("failed to convert wast to utf8");

        let mut lexer = Lexer::new(wast);
        // we need to allow confusing unicode characters since they are technically valid wasm
        lexer.allow_confusing_unicode(true);

        let buf = ParseBuffer::new_with_lexer(lexer).expect("failed to create parse buffer");
        let wast_data = wast::parser::parse::<Wast>(&buf).expect("failed to parse wat");

        let mut last_module: Option<TinyWasmModule> = None;
        debug!("running {} tests for group: {}", wast_data.directives.len(), group_name);
        for (i, directive) in wast_data.directives.into_iter().enumerate() {
            let span = directive.span();
            use wast::WastDirective::*;
            let name = format!("{}-{}", group_name, i);

            debug!("directive: {:?}", directive);

            match directive {
                Wat(mut module) => {
                    debug!("got wat module");

                    let result = catch_unwind_silent(move || parse_module_bytes(&module.encode().unwrap()))
                        .map_err(|e| eyre!("failed to parse module: {:?}", e))
                        .and_then(|res| res);

                    match &result {
                        Err(_) => last_module = None,
                        Ok(m) => last_module = Some(m.clone()),
                    }

                    if let Err(err) = &result {
                        debug!("failed to parse module: {:?}", err)
                    }

                    test_group.add_result(&format!("{}-parse", name), span, result.map(|_| ()));
                }

                AssertMalformed {
                    span,
                    mut module,
                    message: _,
                } => {
                    let Ok(module) = module.encode() else {
                        test_group.add_result(&format!("{}-malformed", name), span, Ok(()));
                        continue;
                    };

                    let res = catch_unwind_silent(|| parse_module_bytes(&module))
                        .map_err(|e| eyre!("failed to parse module: {:?}", e))
                        .and_then(|res| res);

                    test_group.add_result(
                        &format!("{}-malformed", name),
                        span,
                        match res {
                            Ok(_) => Err(eyre!("expected module to be malformed")),
                            Err(_) => Ok(()),
                        },
                    );
                }

                AssertInvalid {
                    span,
                    mut module,
                    message: _,
                } => {
                    let res = catch_unwind_silent(move || parse_module_bytes(&module.encode().unwrap()))
                        .map_err(|e| eyre!("failed to parse module: {:?}", e))
                        .and_then(|res| res);

                    test_group.add_result(
                        &format!("{}-invalid", name),
                        span,
                        match res {
                            Ok(_) => Err(eyre!("expected module to be invalid")),
                            Err(_) => Ok(()),
                        },
                    );
                }

                AssertTrap { exec, message: _, span } => {
                    let res: Result<tinywasm::Result<()>, _> = catch_unwind_silent(|| {
                        let (module, name, args) = match exec {
                            wast::WastExecute::Wat(_wat) => {
                                panic!("wat not supported");
                            }
                            wast::WastExecute::Get { module: _, global: _ } => {
                                panic!("wat not supported");
                            }
                            wast::WastExecute::Invoke(invoke) => (last_module.as_ref(), invoke.name, invoke.args),
                        };
                        let args = args
                            .into_iter()
                            .map(wastarg2tinywasmvalue)
                            .collect::<Result<Vec<_>>>()
                            .expect("failed to convert args");

                        exec_fn(module, name, &args).map(|_| ())
                    });

                    match res {
                        Err(err) => test_group.add_result(
                            &format!("{}-trap", name),
                            span,
                            Err(eyre!("test panicked: {:?}", err)),
                        ),
                        Ok(Err(tinywasm::Error::Trap(_))) => {
                            test_group.add_result(&format!("{}-trap", name), span, Ok(()))
                        }
                        Ok(Err(err)) => test_group.add_result(
                            &format!("{}-trap", name),
                            span,
                            Err(eyre!("expected trap, got error: {:?}", err)),
                        ),
                        Ok(Ok(())) => {
                            test_group.add_result(&format!("{}-trap", name), span, Err(eyre!("expected trap, got ok")))
                        }
                    }
                }

                AssertReturn { span, exec, results } => {
                    let res: Result<Result<()>, _> = catch_unwind_silent(|| {
                        let invoke = match exec {
                            wast::WastExecute::Wat(_) => {
                                error!("wat not supported");
                                return Err(eyre!("wat not supported"));
                            }
                            wast::WastExecute::Get { module: _, global: _ } => return Err(eyre!("get not supported")),
                            wast::WastExecute::Invoke(invoke) => invoke,
                        };

                        debug!("invoke: {:?}", invoke);
                        let args = invoke
                            .args
                            .into_iter()
                            .map(wastarg2tinywasmvalue)
                            .collect::<Result<Vec<_>>>()
                            .map_err(|e| {
                                error!("failed to convert args: {:?}", e);
                                e
                            })?;

                        let outcomes = exec_fn(last_module.as_ref(), invoke.name, &args).map_err(|e| {
                            error!("failed to execute function: {:?}", e);
                            e
                        })?;

                        debug!("outcomes: {:?}", outcomes);

                        let expected = results
                            .into_iter()
                            .map(wastret2tinywasmvalue)
                            .collect::<Result<Vec<_>>>()
                            .map_err(|e| {
                                error!("failed to convert expected results: {:?}", e);
                                e
                            })?;

                        debug!("expected: {:?}", expected);

                        if outcomes.len() != expected.len() {
                            return Err(eyre!(
                                "span: {:?} expected {} results, got {}",
                                span,
                                expected.len(),
                                outcomes.len()
                            ));
                        }

                        println!("expected: {:?}", expected);
                        println!("outcomes: {:?}", outcomes);

                        outcomes
                            .iter()
                            .zip(expected)
                            .enumerate()
                            .try_for_each(|(i, (outcome, exp))| {
                                (outcome.eq_loose(&exp)).then_some(()).ok_or_else(|| {
                                    eyre!(
                                        "span: {:?}: result {} did not match: {:?} != {:?}",
                                        span.linecol_in(&wast),
                                        i,
                                        outcome,
                                        exp
                                    )
                                })
                            })
                    });

                    let res = res
                        .map_err(|e| {
                            error!("test panicked: {:?}", e);
                            eyre!("test panicked: {:?}", e.downcast_ref::<&str>())
                        })
                        .and_then(|r| r);

                    test_group.add_result(&format!("{}-return", name), span, res);
                }
                _ => test_group.add_result(&format!("{}-unknown", name), span, Err(eyre!("unsupported directive"))),
            }
        }

        Ok(())
    }
}
