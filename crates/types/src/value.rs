use core::fmt::Debug;

use crate::{ConstInstruction, ExternAddr, FuncAddr};

/// A WebAssembly value.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#value-types>
#[derive(Clone, Copy, PartialEq)]
pub enum WasmValue {
    // Num types
    /// A 32-bit integer.
    I32(i32),
    /// A 64-bit integer.
    I64(i64),
    /// A 32-bit float.
    F32(f32),
    /// A 64-bit float.
    F64(f64),
    // /// A 128-bit vector
    V128(u128),

    RefExtern(ExternAddr),
    RefFunc(FuncAddr),
    RefNull(ValType),
}

impl WasmValue {
    #[inline]
    pub fn const_instr(&self) -> ConstInstruction {
        match self {
            Self::I32(i) => ConstInstruction::I32Const(*i),
            Self::I64(i) => ConstInstruction::I64Const(*i),
            Self::F32(i) => ConstInstruction::F32Const(*i),
            Self::F64(i) => ConstInstruction::F64Const(*i),
            Self::RefFunc(i) => ConstInstruction::RefFunc(*i),
            Self::RefNull(ty) => ConstInstruction::RefNull(*ty),

            // Self::RefExtern(addr) => ConstInstruction::RefExtern(*addr),
            _ => unimplemented!("no const_instr for {:?}", self),
        }
    }

    /// Get the default value for a given type.
    #[inline]
    pub fn default_for(ty: ValType) -> Self {
        match ty {
            ValType::I32 => Self::I32(0),
            ValType::I64 => Self::I64(0),
            ValType::F32 => Self::F32(0.0),
            ValType::F64 => Self::F64(0.0),
            ValType::V128 => Self::V128(0),
            ValType::RefFunc => Self::RefNull(ValType::RefFunc),
            ValType::RefExtern => Self::RefNull(ValType::RefExtern),
        }
    }

    #[inline]
    pub fn eq_loose(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::I32(a), Self::I32(b)) => a == b,
            (Self::I64(a), Self::I64(b)) => a == b,
            (Self::RefNull(v), Self::RefNull(v2)) => v == v2,
            (Self::RefExtern(addr), Self::RefExtern(addr2)) => addr == addr2,
            (Self::RefFunc(addr), Self::RefFunc(addr2)) => addr == addr2,
            (Self::F32(a), Self::F32(b)) => {
                if a.is_nan() && b.is_nan() {
                    true // Both are NaN, treat them as equal
                } else {
                    a.to_bits() == b.to_bits()
                }
            }
            (Self::F64(a), Self::F64(b)) => {
                if a.is_nan() && b.is_nan() {
                    true // Both are NaN, treat them as equal
                } else {
                    a.to_bits() == b.to_bits()
                }
            }
            _ => false,
        }
    }

    #[doc(hidden)]
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::I32(i) => Some(*i),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(i) => Some(*i),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(i) => Some(*i),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F64(i) => Some(*i),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_v128(&self) -> Option<u128> {
        match self {
            Self::V128(i) => Some(*i),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_ref_extern(&self) -> Option<ExternAddr> {
        match self {
            Self::RefExtern(addr) => Some(*addr),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_ref_func(&self) -> Option<FuncAddr> {
        match self {
            Self::RefFunc(addr) => Some(*addr),
            _ => None,
        }
    }

    #[doc(hidden)]
    pub fn as_ref_null(&self) -> Option<ValType> {
        match self {
            Self::RefNull(ty) => Some(*ty),
            _ => None,
        }
    }
}

#[cold]
fn cold() {}

impl Debug for WasmValue {
    fn fmt(&self, f: &mut alloc::fmt::Formatter<'_>) -> alloc::fmt::Result {
        match self {
            WasmValue::I32(i) => write!(f, "i32({i})"),
            WasmValue::I64(i) => write!(f, "i64({i})"),
            WasmValue::F32(i) => write!(f, "f32({i})"),
            WasmValue::F64(i) => write!(f, "f64({i})"),
            WasmValue::V128(i) => write!(f, "v128({i:?})"),
            WasmValue::RefExtern(addr) => write!(f, "ref.extern({addr:?})"),
            WasmValue::RefFunc(addr) => write!(f, "ref.func({addr:?})"),
            WasmValue::RefNull(ty) => write!(f, "ref.null({ty:?})"),
        }
    }
}

impl WasmValue {
    /// Get the type of a [`WasmValue`]
    #[inline]
    pub fn val_type(&self) -> ValType {
        match self {
            Self::I32(_) => ValType::I32,
            Self::I64(_) => ValType::I64,
            Self::F32(_) => ValType::F32,
            Self::F64(_) => ValType::F64,
            Self::V128(_) => ValType::V128,
            Self::RefExtern(_) => ValType::RefExtern,
            Self::RefFunc(_) => ValType::RefFunc,
            Self::RefNull(ty) => *ty,
        }
    }
}

/// a wrapper for `funcref` value for use in typed wrappers
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WasmFuncRef(FuncAddr);

/// a wrapper for `externref` value for use in typed wrappers
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WasmExternRef(ExternAddr);

macro_rules! impl_newtype_from_into {
    ($wrapper:ty, $underlying:ty) => {
        impl From<$underlying> for $wrapper {
            fn from(value: $underlying) -> Self {
                Self(value)
            }
        }
        impl From<$wrapper> for $underlying {
            fn from(value: $wrapper) -> Self {
                value.0
            }
        }
    };
}

impl_newtype_from_into!(WasmFuncRef, FuncAddr);
impl_newtype_from_into!(WasmExternRef, ExternAddr);

/// Type of a WebAssembly value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "archive", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub enum ValType {
    /// A 32-bit integer.
    I32,
    /// A 64-bit integer.
    I64,
    /// A 32-bit float.
    F32,
    /// A 64-bit float.
    F64,
    /// A 128-bit vector
    V128,
    /// A reference to a function.
    RefFunc,
    /// A reference to an external value.
    RefExtern,
}

impl ValType {
    #[inline]
    pub fn default_value(&self) -> WasmValue {
        WasmValue::default_for(*self)
    }

    #[inline]
    pub fn is_simd(&self) -> bool {
        matches!(self, ValType::V128)
    }
}

macro_rules! impl_conversion_for_wasmvalue {
    ($($t:ty => $variant:ident),*) => {
        $(
            // Implementing From<$t> for WasmValue
            impl From<$t> for WasmValue {
                #[inline]
                fn from(i: $t) -> Self {
                    Self::$variant(i.into())
                }
            }

            // Implementing TryFrom<WasmValue> for $t
            impl TryFrom<WasmValue> for $t {
                type Error = ();

                #[inline]
                fn try_from(value: WasmValue) -> Result<Self, Self::Error> {
                    if let WasmValue::$variant(i) = value {
                        Ok(i.into())
                    } else {
                        cold();
                        Err(())
                    }
                }
            }
        )*
    }
}

impl_conversion_for_wasmvalue! { i32 => I32, i64 => I64, f32 => F32, f64 => F64, u128 => V128, WasmFuncRef=>RefFunc, WasmExternRef=>RefExtern }
