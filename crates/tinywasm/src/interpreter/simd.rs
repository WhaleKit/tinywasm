use tinywasm_types::SimdInstruction;

use crate::Result;

#[cfg(not(feature = "std"))]
#[allow(unused_imports)]
use super::no_std_floats::NoStdFloatExt;
use super::{executor::Executor, Value128};

#[inline(always)]
pub(crate) fn exec_next_simd(e: &mut Executor<'_, '_>, op: SimdInstruction) -> Result<()> {
    match op {
        // unops
        SimdInstruction::V128Not => e.stack.values.replace_top_same(|a: Value128| Ok(!a))?,
        // binops
        SimdInstruction::V128And => e.stack.values.calculate_same(|a: Value128, b: Value128| Ok(a & b))?,
        SimdInstruction::V128AndNot => e.stack.values.calculate_same(|a: Value128, b: Value128| Ok(a & !b))?,
        SimdInstruction::V128Or => e.stack.values.calculate_same(|a: Value128, b: Value128| Ok(a | b))?,
        SimdInstruction::V128Xor => e.stack.values.calculate_same(|a: Value128, b: Value128| Ok(a ^ b))?,
        // ternops
        SimdInstruction::V128Bitselect => {
            let c: Value128 = e.stack.values.pop();
            e.stack.values.calculate(|a: Value128, b: Value128| Ok((a & b) | (!a & c)))?;
        }
        // shifts
        _ => {}
    }
    Ok(())
}

// trait SimdExt {}
// impl SimdExt for Value128 {}
