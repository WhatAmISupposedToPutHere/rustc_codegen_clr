use serde::{Deserialize, Serialize};

use super::field::{StaticFieldDesc, StaticFieldIdx};
use super::{bimap::HashWrapper, Assembly, Const, Int, MethodRefIdx, SigIdx, TypeIdx};
use super::{FieldDesc, FieldIdx, Float};
use crate::r#type::Type as V1Type;
use crate::{
    cil_node::CILNode as V1Node,
    v2::{ClassRef, FnSig, MethodRef, Type},
};
#[derive(Hash, PartialEq, Eq, Clone, Default, Debug, Serialize, Deserialize)]
pub struct NodeIdx(u64);
impl HashWrapper for NodeIdx {
    fn from_hash(val: u64) -> Self {
        Self(val)
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum CILNode {
    Const(Const),
    BinOp(NodeIdx, NodeIdx, BinOp),
    UnOp(NodeIdx, UnOp),
    LdLoc(u64),
    LdLocA(u64),
    LdArg(u64),
    LdArgA(u64),
    Call(Box<(MethodRefIdx, Box<[NodeIdx]>)>),
    IntCast {
        input: NodeIdx,
        target: Int,
        extend: ExtendKind,
    },
    FloatCast {
        input: NodeIdx,
        target: Float,
        is_signed: bool,
    },
    RefToPtr(NodeIdx),
    /// Changes the type of a pointer to `PtrCastRes`
    PtrCast(NodeIdx, Box<PtrCastRes>),
    /// Loads the address of a field at `addr`
    LdFieldAdress {
        addr: NodeIdx,
        field: FieldIdx,
    },
    /// Loads the value of a field at `addr`
    LdField {
        addr: NodeIdx,
        field: FieldIdx,
    },
    /// Loads a value of `tpe` at `addr`
    LdInd {
        addr: NodeIdx,
        tpe: TypeIdx,
        volitale: bool,
    },
    /// Calcualtes the size of a type.
    SizeOf(TypeIdx),
    /// Gets the currenrt exception, if it exisits. UB outside an exception handler.
    GetException,
    /// Checks if the object is an instace of a class.
    IsInst(NodeIdx, TypeIdx),
    /// Casts  the object to instace of a clsass.
    CheckedCast(NodeIdx, TypeIdx),
    /// Calls fn pointer with args
    CallI(Box<(NodeIdx, SigIdx, Box<[NodeIdx]>)>),
    /// Allocates memory from a local pool. It will get freed when this function return
    LocAlloc {
        size: NodeIdx,
    },
    /// Loads a static field at descr
    LdStaticField(StaticFieldIdx),
    /// Loads a pointer to a function
    LdFtn(MethodRefIdx),
    /// Loads a "type token"
    LdTypeToken(TypeIdx),
    /// Gets the length of a platform array
    LdLen(NodeIdx),
    /// Allocates a local buffer sizeof type, and aligned to algin.
    LocAllocAlgined {
        tpe: TypeIdx,
        align: u64,
    },
    /// Loads a reference to array element at index.
    LdElelemRef {
        array: NodeIdx,
        index: NodeIdx,
    },
    /// Turns a managed reference to object into type
    UnboxAny {
        object: NodeIdx,
        tpe: TypeIdx,
    },
}

impl std::hash::Hash for CILNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            CILNode::Const(cst) => {
                "Const".hash(state);
                cst.hash(state);
            }
            CILNode::BinOp(lhs, rhs, op) => {
                "BinOp".hash(state);
                lhs.hash(state);
                rhs.hash(state);
                op.hash(state);
            }
            CILNode::UnOp(val, op) => {
                "UnOp".hash(state);
                val.hash(state);
                op.hash(state);
            }
            CILNode::LdLoc(loc) => {
                "LdLoc".hash(state);
                loc.hash(state)
            }
            CILNode::LdLocA(loc) => {
                "LdLocA".hash(state);
                loc.hash(state)
            }
            CILNode::LdArg(arg) => {
                "LdArg".hash(state);
                arg.hash(state)
            }
            CILNode::LdArgA(arg) => {
                "LdArgA".hash(state);
                arg.hash(state)
            }
            CILNode::Call(call) => {
                "Call".hash(state);
                call.hash(state);
            }
            CILNode::IntCast {
                input,
                target,
                extend,
            } => {
                "IntCast".hash(state);
                input.hash(state);
                target.hash(state);
                extend.hash(state);
            }
            CILNode::FloatCast {
                input,
                target,
                is_signed,
            } => {
                "FloatCast".hash(state);
                input.hash(state);
                target.hash(state);
                is_signed.hash(state);
            }
            CILNode::RefToPtr(ptr) => {
                "RefToPtr".hash(state);
                ptr.hash(state);
            }
            CILNode::PtrCast(ptr, new_tpe) => {
                "PtrCast".hash(state);
                ptr.hash(state);
                new_tpe.hash(state);
            }
            CILNode::LdFieldAdress { addr, field } => {
                "LdFieldAdress".hash(state);
                addr.hash(state);
                field.hash(state);
            }
            CILNode::LdField { addr, field } => {
                "LdField".hash(state);
                addr.hash(state);
                field.hash(state);
            }
            CILNode::LdInd {
                addr,
                tpe,
                volitale,
            } => {
                "LdInd".hash(state);
                addr.hash(state);
                tpe.hash(state);
                volitale.hash(state);
            }
            CILNode::SizeOf(tpe) => {
                "SizeOf".hash(state);
                tpe.hash(state);
            }
            CILNode::GetException => "GetException".hash(state),
            CILNode::IsInst(val, tpe) => {
                "IsInst".hash(state);
                val.hash(state);
                tpe.hash(state);
            }
            CILNode::CheckedCast(val, tpe) => {
                "CheckedCast".hash(state);
                val.hash(state);
                tpe.hash(state);
            }
            CILNode::CallI(call) => {
                "CallI".hash(state);
                call.hash(state);
            }
            CILNode::LocAlloc { size } => {
                "LocAlloc".hash(state);
                size.hash(state);
            }
            CILNode::LocAllocAlgined { tpe, align: algin } => {
                "LocAllocAlgined".hash(state);
                tpe.hash(state);
                algin.hash(state);
            }
            CILNode::LdStaticField(fld) => {
                "LdStaticField".hash(state);
                fld.hash(state);
            }
            CILNode::LdFtn(site) => {
                "LdFtn".hash(state);
                site.hash(state);
            }
            CILNode::LdTypeToken(tpe) => {
                "LdTypeToken".hash(state);
                tpe.hash(state);
            }
            CILNode::LdLen(val) => {
                "LdLen".hash(state);
                val.hash(state);
            }
            CILNode::LdElelemRef { array, index } => {
                "LdElelemRef".hash(state);
                array.hash(state);
                index.hash(state);
            }
            CILNode::UnboxAny { object, tpe } => {
                "UnboxAny".hash(state);
                object.hash(state);
                tpe.hash(state);
            }
        }
    }
}
#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum PtrCastRes {
    Ptr(TypeIdx),
    Ref(TypeIdx),
    FnPtr(SigIdx),
    USize,
    ISize,
}
#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]

pub enum ExtendKind {
    ZeroExtend,
    SignExtend,
}
#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum MethodKind {
    Static,
    Instance,
    Virtual,
    Constructor,
}
#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum UnOp {
    Not,
    Neg,
}
#[derive(Hash, PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]

pub enum BinOp {
    Add,
    Eq,
    Sub,
    Mul,
    LtUn,
    Lt,
    GtUn,
    Gt,
    Or,
    XOr,
    And,
    Rem,
    RemUn,
    Shl,
    Shr,
    ShrUn,
    DivUn,
    Div,
}
impl CILNode {
    pub fn from_v1(v1: &V1Node, asm: &mut Assembly) -> Self {
        match v1 {
            // Varaible access
            V1Node::LDArg(arg) => CILNode::LdArg(*arg as u64),
            V1Node::LDLoc(arg) => CILNode::LdLoc(*arg as u64),
            V1Node::LDArgA(arg) => CILNode::LdArgA(*arg as u64),
            V1Node::LDLocA(arg) => CILNode::LdLocA(*arg as u64),
            // Ptr deref
            V1Node::LDIndBool { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Bool),
                    volitale: false,
                }
            }
            V1Node::LDIndU8 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::U8)),
                    volitale: false,
                }
            }
            V1Node::LDIndU16 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::U16)),
                    volitale: false,
                }
            }
            V1Node::LDIndU32 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::U32)),
                    volitale: false,
                }
            }
            V1Node::LDIndU64 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::U64)),
                    volitale: false,
                }
            }
            V1Node::LDIndUSize { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::USize)),
                    volitale: false,
                }
            }
            V1Node::LDIndI8 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::I8)),
                    volitale: false,
                }
            }
            V1Node::LDIndI16 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::I16)),
                    volitale: false,
                }
            }
            V1Node::LDIndI32 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::I32)),
                    volitale: false,
                }
            }
            V1Node::LDIndI64 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::I64)),
                    volitale: false,
                }
            }
            V1Node::LDIndISize { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Int(Int::ISize)),
                    volitale: false,
                }
            }
            V1Node::LDIndF32 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Float(Float::F32)),
                    volitale: false,
                }
            }
            V1Node::LDIndF64 { ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(Type::Float(Float::F64)),
                    volitale: false,
                }
            }
            V1Node::LdObj { ptr, obj } => {
                let obj = Type::from_v1(obj, asm);
                let ptr = Self::from_v1(ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(obj),
                    volitale: false,
                }
            }
            V1Node::LDIndPtr { ptr, loaded_ptr } => {
                let ptr = Self::from_v1(ptr, asm);
                let loaded_ptr = Type::from_v1(loaded_ptr, asm);
                Self::LdInd {
                    addr: asm.node_idx(ptr),
                    tpe: asm.type_idx(loaded_ptr),
                    volitale: false,
                }
            }
            // Casts
            V1Node::ZeroExtendToU64(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::U64,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::SignExtendToU64(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::U64,
                    extend: ExtendKind::SignExtend,
                }
            }
            V1Node::ZeroExtendToUSize(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::USize,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::SignExtendToUSize(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::USize,
                    extend: ExtendKind::SignExtend,
                }
            }
            V1Node::ConvU8(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::U8,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::ConvU16(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::U16,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::ConvU32(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::U32,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::SignExtendToI64(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::I64,
                    extend: ExtendKind::SignExtend,
                }
            }
            V1Node::ZeroExtendToISize(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::ISize,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::SignExtendToISize(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::ISize,
                    extend: ExtendKind::SignExtend,
                }
            }
            V1Node::ConvI8(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::I8,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::ConvI16(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::I16,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::ConvI32(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::IntCast {
                    input: asm.node_idx(node),
                    target: Int::I32,
                    extend: ExtendKind::ZeroExtend,
                }
            }
            V1Node::ConvF32(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::FloatCast {
                    input: asm.node_idx(node),
                    target: Float::F32,
                    is_signed: true,
                }
            }
            V1Node::ConvF64(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::FloatCast {
                    input: asm.node_idx(node),
                    target: Float::F64,
                    is_signed: true,
                }
            }
            V1Node::ConvF64Un(inner) => {
                let node = Self::from_v1(inner, asm);
                CILNode::FloatCast {
                    input: asm.node_idx(node),
                    target: Float::F64,
                    is_signed: false,
                }
            }
            V1Node::MRefToRawPtr(inner) => {
                let raw = Self::from_v1(inner, asm);
                CILNode::RefToPtr(asm.node_idx(raw))
            }
            V1Node::CastPtr { val, new_ptr } => {
                let val = Self::from_v1(val, asm);

                let ptr = match &**new_ptr {
                    V1Type::USize => PtrCastRes::USize,
                    V1Type::ISize => PtrCastRes::ISize,
                    V1Type::Ptr(inner) => {
                        let inner = Type::from_v1(inner, asm);
                        PtrCastRes::Ptr(asm.type_idx(inner))
                    }
                    V1Type::ManagedReference(inner) => {
                        let inner = Type::from_v1(inner, asm);
                        PtrCastRes::Ref(asm.type_idx(inner))
                    }
                    V1Type::DelegatePtr(sig) => {
                        let sig = FnSig::from_v1(sig, asm);
                        let sig = asm.sig_idx(sig);
                        PtrCastRes::FnPtr(sig)
                    }
                    _ => panic!("Type {new_ptr:?} is not a pointer."),
                };
                CILNode::PtrCast(asm.node_idx(val), Box::new(ptr))
            }
            // Binops
            V1Node::Add(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Add)
            }
            V1Node::Sub(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Sub)
            }
            V1Node::Mul(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Mul)
            }
            V1Node::Eq(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Eq)
            }
            V1Node::Or(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Or)
            }
            V1Node::XOr(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::XOr)
            }
            V1Node::And(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::And)
            }
            V1Node::LtUn(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::LtUn)
            }
            V1Node::Lt(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Lt)
            }
            V1Node::GtUn(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::GtUn)
            }
            V1Node::Gt(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Gt)
            }
            V1Node::Rem(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Rem)
            }
            V1Node::RemUn(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::RemUn)
            }
            V1Node::Shl(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Shl)
            }
            V1Node::Shr(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Shr)
            }
            V1Node::ShrUn(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::ShrUn)
            }
            V1Node::Div(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::Div)
            }
            V1Node::DivUn(lhs, rhs) => {
                let lhs = Self::from_v1(lhs, asm);
                let rhs = Self::from_v1(rhs, asm);
                Self::BinOp(asm.node_idx(lhs), asm.node_idx(rhs), BinOp::DivUn)
            }
            // Unops
            V1Node::Not(val) => {
                let val = Self::from_v1(val, asm);
                Self::UnOp(asm.node_idx(val), UnOp::Not)
            }
            V1Node::Neg(val) => {
                let val = Self::from_v1(val, asm);
                Self::UnOp(asm.node_idx(val), UnOp::Neg)
            }
            // Field access
            V1Node::LDField { addr, field } => {
                let field = FieldDesc::from_v1(field, asm);
                let field = asm.field_idx(field);
                let addr = Self::from_v1(addr, asm);
                Self::LdField {
                    addr: asm.node_idx(addr),
                    field,
                }
            }
            V1Node::LDFieldAdress { addr, field } => {
                let field = FieldDesc::from_v1(field, asm);
                let field = asm.field_idx(field);
                let addr = Self::from_v1(addr, asm);
                Self::LdField {
                    addr: asm.node_idx(addr),
                    field,
                }
            }
            // Calls
            V1Node::Call(callargs) => {
                let args: Box<[_]> = callargs
                    .args
                    .iter()
                    .map(|arg| {
                        let node = Self::from_v1(arg, asm);
                        asm.node_idx(node)
                    })
                    .collect();
                let sig = FnSig::from_v1(callargs.site.signature(), asm);
                let sig = asm.sig_idx(sig);
                let generics: Box<[_]> = callargs
                    .site
                    .generics()
                    .iter()
                    .map(|gen| Type::from_v1(gen, asm))
                    .collect();
                let class = callargs.site.class().map(|dt| {
                    let cref = ClassRef::from_v1(dt, asm);
                    asm.class_idx(cref)
                });
                let name = asm.alloc_string(callargs.site.name());
                let method_ref = if callargs.site.is_static() {
                    MethodRef::new(class, name, sig, MethodKind::Static, generics)
                } else {
                    MethodRef::new(class, name, sig, MethodKind::Instance, generics)
                };
                let method_ref = asm.methodref_idx(method_ref);
                Self::Call(Box::new((method_ref, args)))
            }
            V1Node::CallVirt(callargs) => {
                let args: Box<[_]> = callargs
                    .args
                    .iter()
                    .map(|arg| {
                        let node = Self::from_v1(arg, asm);
                        asm.node_idx(node)
                    })
                    .collect();
                let sig = FnSig::from_v1(callargs.site.signature(), asm);
                let sig = asm.sig_idx(sig);
                let generics: Box<[_]> = callargs
                    .site
                    .generics()
                    .iter()
                    .map(|gen| Type::from_v1(gen, asm))
                    .collect();
                let class = callargs.site.class().map(|dt| {
                    let cref = ClassRef::from_v1(dt, asm);
                    asm.class_idx(cref)
                });
                let name = asm.alloc_string(callargs.site.name());
                assert!(!callargs.site.is_static());
                let method_ref = MethodRef::new(class, name, sig, MethodKind::Virtual, generics);
                let method_ref = asm.methodref_idx(method_ref);
                Self::Call(Box::new((method_ref, args)))
            }
            V1Node::NewObj(callargs) => {
                let args: Box<[_]> = callargs
                    .args
                    .iter()
                    .map(|arg| {
                        let node = Self::from_v1(arg, asm);
                        asm.node_idx(node)
                    })
                    .collect();
                let sig = FnSig::from_v1(callargs.site.signature(), asm);
                let sig = asm.sig_idx(sig);
                let generics: Box<[_]> = callargs
                    .site
                    .generics()
                    .iter()
                    .map(|gen| Type::from_v1(gen, asm))
                    .collect();
                let class = callargs.site.class().map(|dt| {
                    let cref = ClassRef::from_v1(dt, asm);
                    asm.class_idx(cref)
                });
                let name = asm.alloc_string(callargs.site.name());
                assert!(
                    !callargs.site.is_static(),
                    "Newobj site invalid(is static):{:?}",
                    callargs.site
                );
                let method_ref =
                    MethodRef::new(class, name, sig, MethodKind::Constructor, generics);
                let method_ref = asm.methodref_idx(method_ref);
                Self::Call(Box::new((method_ref, args)))
            }
            // Special
            V1Node::GetException => Self::GetException,
            // Consts
            V1Node::LdStr(string) => {
                let string = asm.alloc_string(string.clone());
                Const::PlatformString(string).into()
            }
            V1Node::SizeOf(tpe) => {
                let tpe = Type::from_v1(tpe, asm);
                Self::SizeOf(asm.type_idx(tpe))
            }
            V1Node::LDTypeToken(tpe) => {
                let tpe = Type::from_v1(tpe, asm);
                Self::LdTypeToken(asm.type_idx(tpe))
            }
            V1Node::LdcU64(val) => Const::U64(*val).into(),
            V1Node::LdcU32(val) => Const::U32(*val).into(),
            V1Node::LdcU16(val) => Const::U16(*val).into(),
            V1Node::LdcU8(val) => Const::U8(*val).into(),
            V1Node::LdcI64(val) => Const::I64(*val).into(),
            V1Node::LdcI32(val) => Const::I32(*val).into(),
            V1Node::LdcI16(val) => Const::I16(*val).into(),
            V1Node::LdcI8(val) => Const::I8(*val).into(),
            V1Node::LdFalse => Const::Bool(false).into(),
            V1Node::LdTrue => Const::Bool(true).into(),
            V1Node::LdcF64(val) => Const::F64(*val).into(),
            V1Node::LdcF32(val) => Const::F32(*val).into(),
            // Special
            V1Node::IsInst(combined) => {
                let (val, tpe) = combined.as_ref();
                let tpe = ClassRef::from_v1(tpe, asm);
                let tpe = asm.class_idx(tpe);
                let tpe = asm.type_idx(tpe.into());
                let val = Self::from_v1(val, asm);

                Self::IsInst(asm.node_idx(val), tpe)
            }
            V1Node::CheckedCast(combined) => {
                let (val, tpe) = combined.as_ref();
                let tpe = ClassRef::from_v1(tpe, asm);
                let tpe = asm.class_idx(tpe);
                let tpe = asm.type_idx(tpe.into());
                let val = Self::from_v1(val, asm);

                Self::CheckedCast(asm.node_idx(val), tpe)
            }
            V1Node::CallI(sig_ptr_args) => {
                let sig = FnSig::from_v1(&sig_ptr_args.0, asm);
                let sig = asm.sig_idx(sig);
                let ptr = Self::from_v1(&sig_ptr_args.1, asm);
                let ptr = asm.node_idx(ptr);
                let args: Box<[_]> = sig_ptr_args
                    .2
                    .iter()
                    .map(|arg| {
                        let arg = Self::from_v1(arg, asm);
                        asm.node_idx(arg)
                    })
                    .collect();
                Self::CallI(Box::new((ptr, sig, args)))
            }
            V1Node::LocAlloc { size } => {
                let size = Self::from_v1(size, asm);
                let size = asm.node_idx(size);
                CILNode::LocAlloc { size }
            }
            V1Node::LocAllocAligned { tpe, align } => {
                let tpe = Type::from_v1(tpe, asm);
                let tpe = asm.type_idx(tpe);
                CILNode::LocAllocAlgined { tpe, align: *align }
            }
            V1Node::LDStaticField(sfld) => {
                let sfld = StaticFieldDesc::from_v1(sfld, asm);
                Self::LdStaticField(asm.sfld_idx(sfld))
            }
            V1Node::LDFtn(site) => {
                let sig = FnSig::from_v1(site.signature(), asm);
                let sig = asm.sig_idx(sig);
                let generics: Box<[_]> = site
                    .generics()
                    .iter()
                    .map(|gen| Type::from_v1(gen, asm))
                    .collect();
                let class = site.class().map(|dt| {
                    let cref = ClassRef::from_v1(dt, asm);
                    asm.class_idx(cref)
                });
                let name = asm.alloc_string(site.name());

                let method_ref = if site.is_static() {
                    MethodRef::new(class, name, sig, MethodKind::Static, generics)
                } else {
                    MethodRef::new(class, name, sig, MethodKind::Instance, generics)
                };
                let method_ref = asm.methodref_idx(method_ref);
                Self::LdFtn(method_ref)
            }
            V1Node::Volatile(inner) => {
                let mut tmp = Self::from_v1(inner, asm);
                match &mut tmp {
                    Self::LdInd { volitale, .. } => *volitale = true,
                    _ => panic!(),
                }
                tmp
            }
            V1Node::LDLen { arr } => {
                let arr = Self::from_v1(arr, asm);
                let arr = asm.node_idx(arr);
                Self::LdLen(arr)
            }
            V1Node::LDElelemRef { arr, idx } => {
                let arr = Self::from_v1(arr, asm);
                let array = asm.node_idx(arr);
                let idx = Self::from_v1(idx, asm);
                let index = asm.node_idx(idx);
                Self::LdElelemRef { array, index }
            }
            V1Node::UnboxAny(object, tpe) => {
                let object = Self::from_v1(object, asm);
                let object = asm.node_idx(object);
                let tpe = Type::from_v1(tpe, asm);
                let tpe = asm.type_idx(tpe);
                Self::UnboxAny { object, tpe }
            }
            _ => todo!("v1:{v1:?}"),
        }
    }
}
