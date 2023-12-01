use alloc::{format, vec::Vec};
use tinywasm_types::{BlockArgs, Instruction, MemArg, ValType};

use crate::Result;

fn convert_blocktype(blocktype: wasmparser::BlockType) -> BlockArgs {
    use wasmparser::BlockType::*;
    match blocktype {
        Empty => BlockArgs::Empty,
        Type(ty) => BlockArgs::Type(convert_valtype(ty)),
        FuncType(ty) => BlockArgs::FuncType(ty),
    }
}

fn convert_valtype(valtype: wasmparser::ValType) -> ValType {
    use wasmparser::ValType::*;
    match valtype {
        I32 => ValType::I32,
        I64 => ValType::I64,
        F32 => ValType::F32,
        F64 => ValType::F64,
        V128 => ValType::V128,
        FuncRef => ValType::FuncRef,
        ExternRef => ValType::ExternRef,
    }
}

fn convert_memarg(memarg: wasmparser::MemArg) -> MemArg {
    MemArg {
        offset: memarg.offset,
        align: memarg.align,
    }
}

pub fn process_operator(op: wasmparser::Operator<'_>) -> Result<Instruction> {
    use wasmparser::Operator::*;
    let v = match op {
        Unreachable => Instruction::Unreachable,
        Nop => Instruction::Nop,
        Block { blockty } => Instruction::Block(convert_blocktype(blockty)),
        Loop { blockty } => Instruction::Loop(convert_blocktype(blockty)),
        If { blockty } => Instruction::If(convert_blocktype(blockty)),
        Else => Instruction::Else,
        End => Instruction::End,
        Br { relative_depth } => Instruction::Br(relative_depth),
        BrIf { relative_depth } => Instruction::BrIf(relative_depth),
        BrTable { targets } => {
            let default = targets.default();
            let targets = targets
                .targets()
                .map(|t| Ok(t?))
                .collect::<Result<Vec<u32>>>()?;

            Instruction::BrTable(targets, default)
        }
        Return => Instruction::Return,
        Call { function_index } => Instruction::Call(function_index),
        CallIndirect {
            type_index,
            table_index,
            ..
        } => Instruction::CallIndirect(type_index, table_index),
        Drop => Instruction::Drop,
        Select => Instruction::Select,
        LocalGet { local_index } => Instruction::LocalGet(local_index),
        LocalSet { local_index } => Instruction::LocalSet(local_index),
        LocalTee { local_index } => Instruction::LocalTee(local_index),
        GlobalGet { global_index } => Instruction::GlobalGet(global_index),
        GlobalSet { global_index } => Instruction::GlobalSet(global_index),
        MemorySize { .. } => Instruction::MemorySize,
        MemoryGrow { .. } => Instruction::MemoryGrow,
        I32Load { memarg } => Instruction::I32Load(convert_memarg(memarg)),
        I64Load { memarg } => Instruction::I64Load(convert_memarg(memarg)),
        F32Load { memarg } => Instruction::F32Load(convert_memarg(memarg)),
        F64Load { memarg } => Instruction::F64Load(convert_memarg(memarg)),
        I32Load8S { memarg } => Instruction::I32Load8S(convert_memarg(memarg)),
        I32Load8U { memarg } => Instruction::I32Load8U(convert_memarg(memarg)),
        I32Load16S { memarg } => Instruction::I32Load16S(convert_memarg(memarg)),
        I32Load16U { memarg } => Instruction::I32Load16U(convert_memarg(memarg)),
        I64Load8S { memarg } => Instruction::I64Load8S(convert_memarg(memarg)),
        I64Load8U { memarg } => Instruction::I64Load8U(convert_memarg(memarg)),
        I64Load16S { memarg } => Instruction::I64Load16S(convert_memarg(memarg)),
        I64Load16U { memarg } => Instruction::I64Load16U(convert_memarg(memarg)),
        I64Load32S { memarg } => Instruction::I64Load32S(convert_memarg(memarg)),
        I64Load32U { memarg } => Instruction::I64Load32U(convert_memarg(memarg)),
        I32Store { memarg } => Instruction::I32Store(convert_memarg(memarg)),
        I64Store { memarg } => Instruction::I64Store(convert_memarg(memarg)),
        F32Store { memarg } => Instruction::F32Store(convert_memarg(memarg)),
        F64Store { memarg } => Instruction::F64Store(convert_memarg(memarg)),
        I32Store8 { memarg } => Instruction::I32Store8(convert_memarg(memarg)),
        I32Store16 { memarg } => Instruction::I32Store16(convert_memarg(memarg)),
        I64Store8 { memarg } => Instruction::I64Store8(convert_memarg(memarg)),
        I64Store16 { memarg } => Instruction::I64Store16(convert_memarg(memarg)),
        I64Store32 { memarg } => Instruction::I64Store32(convert_memarg(memarg)),
        I32Eqz => Instruction::I32Eqz,
        I32Eq => Instruction::I32Eq,
        I32Ne => Instruction::I32Ne,
        I32LtS => Instruction::I32LtS,
        I32LtU => Instruction::I32LtU,
        I32GtS => Instruction::I32GtS,
        I32GtU => Instruction::I32GtU,
        I32LeS => Instruction::I32LeS,
        I32LeU => Instruction::I32LeU,
        I32GeS => Instruction::I32GeS,
        I32GeU => Instruction::I32GeU,
        I64Eqz => Instruction::I64Eqz,
        I64Eq => Instruction::I64Eq,
        I64Ne => Instruction::I64Ne,
        I64LtS => Instruction::I64LtS,
        I64LtU => Instruction::I64LtU,
        I64GtS => Instruction::I64GtS,
        I64GtU => Instruction::I64GtU,
        I64LeS => Instruction::I64LeS,
        I64LeU => Instruction::I64LeU,
        I64GeS => Instruction::I64GeS,
        I64GeU => Instruction::I64GeU,
        F32Eq => Instruction::F32Eq,
        F32Ne => Instruction::F32Ne,
        F32Lt => Instruction::F32Lt,
        F32Gt => Instruction::F32Gt,
        F32Le => Instruction::F32Le,
        F32Ge => Instruction::F32Ge,
        F64Eq => Instruction::F64Eq,
        F64Ne => Instruction::F64Ne,
        F64Lt => Instruction::F64Lt,
        F64Gt => Instruction::F64Gt,
        F64Le => Instruction::F64Le,
        F64Ge => Instruction::F64Ge,
        I32Clz => Instruction::I32Clz,
        I32Ctz => Instruction::I32Ctz,
        I32Popcnt => Instruction::I32Popcnt,
        I32Add => Instruction::I32Add,
        I32Sub => Instruction::I32Sub,
        I32Mul => Instruction::I32Mul,
        I32DivS => Instruction::I32DivS,
        I32DivU => Instruction::I32DivU,
        I32RemS => Instruction::I32RemS,
        I32RemU => Instruction::I32RemU,
        I32And => Instruction::I32And,
        I32Or => Instruction::I32Or,
        I32Xor => Instruction::I32Xor,
        I32Shl => Instruction::I32Shl,
        I32ShrS => Instruction::I32ShrS,
        I32ShrU => Instruction::I32ShrU,
        I32Rotl => Instruction::I32Rotl,
        I32Rotr => Instruction::I32Rotr,
        I64Clz => Instruction::I64Clz,
        I64Ctz => Instruction::I64Ctz,
        I64Popcnt => Instruction::I64Popcnt,
        I64Add => Instruction::I64Add,
        I64Sub => Instruction::I64Sub,
        I64Mul => Instruction::I64Mul,
        I64DivS => Instruction::I64DivS,
        I64DivU => Instruction::I64DivU,
        I64RemS => Instruction::I64RemS,
        I64RemU => Instruction::I64RemU,
        I64And => Instruction::I64And,
        I64Or => Instruction::I64Or,
        I64Xor => Instruction::I64Xor,
        I64Shl => Instruction::I64Shl,
        I64ShrS => Instruction::I64ShrS,
        I64ShrU => Instruction::I64ShrU,
        I64Rotl => Instruction::I64Rotl,
        I64Rotr => Instruction::I64Rotr,
        F32Abs => Instruction::F32Abs,
        F32Neg => Instruction::F32Neg,
        F32Ceil => Instruction::F32Ceil,
        F32Floor => Instruction::F32Floor,
        F32Trunc => Instruction::F32Trunc,
        F32Nearest => Instruction::F32Nearest,
        F32Sqrt => Instruction::F32Sqrt,
        F32Add => Instruction::F32Add,
        F32Sub => Instruction::F32Sub,
        F32Mul => Instruction::F32Mul,
        F32Div => Instruction::F32Div,
        F32Min => Instruction::F32Min,
        F32Max => Instruction::F32Max,
        F32Copysign => Instruction::F32Copysign,
        F64Abs => Instruction::F64Abs,
        F64Neg => Instruction::F64Neg,
        F64Ceil => Instruction::F64Ceil,
        F64Floor => Instruction::F64Floor,
        F64Trunc => Instruction::F64Trunc,
        F64Nearest => Instruction::F64Nearest,
        F64Sqrt => Instruction::F64Sqrt,
        F64Add => Instruction::F64Add,
        F64Sub => Instruction::F64Sub,
        F64Mul => Instruction::F64Mul,
        F64Div => Instruction::F64Div,
        F64Min => Instruction::F64Min,
        F64Max => Instruction::F64Max,
        F64Copysign => Instruction::F64Copysign,
        I32WrapI64 => Instruction::I32WrapI64,
        I32TruncF32S => Instruction::I32TruncF32S,
        I32TruncF32U => Instruction::I32TruncF32U,
        I32TruncF64S => Instruction::I32TruncF64S,
        I32TruncF64U => Instruction::I32TruncF64U,
        I64ExtendI32S => Instruction::I64ExtendI32S,
        I64ExtendI32U => Instruction::I64ExtendI32U,
        I64TruncF32S => Instruction::I64TruncF32S,
        I64TruncF32U => Instruction::I64TruncF32U,
        I64TruncF64S => Instruction::I64TruncF64S,
        I64TruncF64U => Instruction::I64TruncF64U,
        F32ConvertI32S => Instruction::F32ConvertI32S,
        F32ConvertI32U => Instruction::F32ConvertI32U,
        F32ConvertI64S => Instruction::F32ConvertI64S,
        F32ConvertI64U => Instruction::F32ConvertI64U,
        F32DemoteF64 => Instruction::F32DemoteF64,
        F64ConvertI32S => Instruction::F64ConvertI32S,
        F64ConvertI32U => Instruction::F64ConvertI32U,
        F64ConvertI64S => Instruction::F64ConvertI64S,
        F64ConvertI64U => Instruction::F64ConvertI64U,
        F64PromoteF32 => Instruction::F64PromoteF32,
        I32ReinterpretF32 => Instruction::I32ReinterpretF32,
        I64ReinterpretF64 => Instruction::I64ReinterpretF64,
        F32ReinterpretI32 => Instruction::F32ReinterpretI32,
        F64ReinterpretI64 => Instruction::F64ReinterpretI64,
        _ => {
            return Err(crate::ParseError::UnsupportedOperator(format!(
                "Unsupported instruction: {:?}",
                op
            )))
        }
    };

    Ok(v)
}
