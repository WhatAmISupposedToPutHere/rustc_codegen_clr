use rustc_middle::mir::{BinOp, Operand};
use rustc_middle::ty::{Instance, IntTy, Ty, TyCtxt, TyKind, UintTy};

use crate::cil::{CILOp, CallSite};
use crate::cil_tree::cil_node::CILNode;
use crate::function_sig::FnSig;
use crate::r#type::{DotnetTypeRef, TyCache, Type};
use crate::utilis::compiletime_sizeof;
use crate::{
    add, and, call, conv_i8, conv_u16, conv_u32, conv_u64, conv_u8, conv_usize, div, eq, gt, gt_un, ldc_i32, lt, lt_un, mul, or, size_of, sub
};
/// Preforms an unchecked binary operation.
pub(crate) fn binop_unchecked<'tyctx>(
    binop: BinOp,
    operand_a: &Operand<'tyctx>,
    operand_b: &Operand<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    method: &rustc_middle::mir::Body<'tyctx>,
    method_instance: Instance<'tyctx>,
    tycache: &mut TyCache,
) -> CILNode {
    let ops_a = crate::operand::handle_operand(operand_a, tyctx, method, method_instance, tycache);
    let ops_b = crate::operand::handle_operand(operand_b, tyctx, method, method_instance, tycache);
    let ty_a = operand_a.ty(&method.local_decls, tyctx);
    let ty_b = operand_b.ty(&method.local_decls, tyctx);
    match binop {
        BinOp::Add | BinOp::AddUnchecked => {
            add_unchecked(ty_a, ty_b, tyctx, &method_instance, tycache, ops_a, ops_b)
        }

        BinOp::Sub | BinOp::SubUnchecked => {
            sub_unchecked(ty_a, ty_b, tyctx, &method_instance, tycache, ops_a, ops_b)
        }
        BinOp::Ne => ne_unchecked(ty_a, ops_a, ops_b),
        BinOp::Eq => eq_unchecked(ty_a, ops_a, ops_b),
        BinOp::Lt => lt_unchecked(ty_a, ops_a, ops_b),
        BinOp::Gt => gt_unchecked(ty_a, ops_a, ops_b),
        BinOp::BitAnd => {
            bit_and_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b)
        }
        BinOp::BitOr => {
            bit_or_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b)
        }
        BinOp::BitXor => bit_xor_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),
        BinOp::Rem => rem_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),
        BinOp::Shl => shl_checked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),
        BinOp::ShlUnchecked => 
            shl_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),
        BinOp::Shr =>
            shr_checked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),
        BinOp::ShrUnchecked => 

            shr_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),

        BinOp::Mul | BinOp::MulUnchecked => 
            mul_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),

        BinOp::Div => div_unchecked(ty_a, ty_b, tycache, &method_instance, tyctx, ops_a, ops_b),

        BinOp::Ge => eq!(gt_unchecked(ty_a, ops_a, ops_b), ldc_i32!(0)),
        BinOp::Le => eq!(lt_unchecked(ty_a, ops_a, ops_b), ldc_i32!(0)),
        BinOp::Offset => {
            let pointed_ty = if let TyKind::RawPtr(inner_and_mut) = ty_a.kind() {
                inner_and_mut.ty
            } else {
                todo!("Can't offset pointer of type {ty_a:?}");
            };
            let pointed_ty = crate::utilis::monomorphize(&method_instance, pointed_ty, tyctx);
            let pointed_ty =
                Box::new(tycache.type_from_cache(pointed_ty, tyctx, Some(method_instance)));
            add!(ops_a,mul!(ops_b,conv_usize!(size_of!(pointed_ty))))
            
        } //_ => todo!("Unsupported bionp {binop:?}"),
    }
}
/// Preforms unchecked addition
fn add_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    method_instance: &Instance<'tyctx>,
    tycache: &mut TyCache,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    match ty_a.kind() {
        TyKind::Int(int_ty) => {
            if let IntTy::I128 = int_ty {
                let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
                let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
                call!(
                    CallSite::new(
                        Some(DotnetTypeRef::int_128()),
                        "op_Addition".into(),
                        FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                        true,
                    ),
                    [ops_a, ops_b]
                )
            } else {
                add!(ops_a, ops_b)
            }
        }
        TyKind::Uint(uint_ty) => {
            if let UintTy::U128 = uint_ty {
                let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
                let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
                call!(
                    CallSite::new(
                        Some(DotnetTypeRef::uint_128()),
                        "op_Addition".into(),
                        FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                        true,
                    ),
                    [ops_a, ops_b]
                )
            } else {
                match uint_ty {
                    UintTy::U8 => conv_u8!(add!(ops_a, ops_b)),
                    UintTy::U16 => conv_u16!(add!(ops_a, ops_b)),
                    UintTy::U32 => conv_u32!(add!(ops_a, ops_b)),
                    UintTy::U64 => conv_u64!(add!(ops_a, ops_b)),
                    _ => add!(ops_a, ops_b),
                }
            }
        }
        TyKind::Float(_) => add!(ops_a, ops_b),
        _ => todo!("can't add numbers of types {ty_a} and {ty_b}"),
    }
}
/// Preforms unchecked subtraction
fn sub_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    method_instance: &Instance<'tyctx>,
    tycache: &mut TyCache,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    match ty_a.kind() {
        TyKind::Int(int_ty) => {
            if let IntTy::I128 = int_ty {
                let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
                let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
                call!(
                    CallSite::new(
                        Some(DotnetTypeRef::int_128()),
                        "op_Subtraction".into(),
                        FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                        true,
                    ),
                    [ops_a, ops_b]
                )
            } else {
                sub!(ops_a, ops_b)
            }
        }
        TyKind::Uint(uint_ty) => {
            if let UintTy::U128 = uint_ty {
                let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
                let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
                call!(
                    CallSite::new(
                        Some(DotnetTypeRef::uint_128()),
                        "op_Subtraction".into(),
                        FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                        true,
                    ),
                    [ops_a, ops_b]
                )
            } else {
                sub!(ops_a, ops_b)
            }
        }
        TyKind::Float(_) => sub!(ops_a, ops_b),
        _ => todo!("can't add numbers of types {ty_a} and {ty_b}"),
    }
}
fn ne_unchecked<'tyctx>(ty_a: Ty<'tyctx>, operand_a: CILNode, operand_b: CILNode) -> CILNode {
    //vec![eq_unchecked(ty_a), CILOp::LdcI32(0), CILOp::Eq]
    eq!(eq_unchecked(ty_a, operand_a, operand_b), ldc_i32!(0))
}
pub fn eq_unchecked<'tyctx>(ty_a: Ty<'tyctx>, operand_a: CILNode, operand_b: CILNode) -> CILNode {
    //vec![CILOp::Eq]
    match ty_a.kind() {
        TyKind::Uint(uint) => match uint {
            UintTy::U128 => call!(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_Equality".into(),
                    FnSig::new(&[Type::U128, Type::U128], &Type::Bool),
                    true,
                ),
                [operand_a, operand_b]
            ),
            _ => eq!(operand_a, operand_b),
        },
        TyKind::Int(int) => match int {
            IntTy::I128 => call!(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_Equality".into(),
                    FnSig::new(&[Type::I128, Type::I128], &Type::Bool),
                    true,
                ),
                [operand_a, operand_b]
            ),
            _ => eq!(operand_a, operand_b),
        },
        TyKind::Bool => eq!(operand_a, operand_b),
        TyKind::Char => eq!(operand_a, operand_b),
        TyKind::Float(_) => eq!(operand_a, operand_b),
        TyKind::RawPtr(_) => eq!(operand_a, operand_b),
        _ => panic!("Can't eq type  {ty_a:?}"),
    }
}
fn lt_unchecked<'tyctx>(ty_a: Ty<'tyctx>, operand_a: CILNode, operand_b: CILNode) -> CILNode {
    //return CILOp::Lt;
    match ty_a.kind() {
        TyKind::Uint(uint) => match uint {
            UintTy::U128 => call!(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_LessThan".into(),
                    FnSig::new(&[Type::U128, Type::U128], &Type::Bool),
                    true,
                ),
                [operand_a, operand_b]
            ),
            _ => lt_un!(operand_a, operand_b),
        },
        TyKind::Int(int) => match int {
            IntTy::I128 => call!(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_LessThan".into(),
                    FnSig::new(&[Type::I128, Type::I128], &Type::Bool),
                    true,
                ),
                [operand_a, operand_b]
            ),
            _ => lt!(operand_a, operand_b),
        },
        TyKind::Bool => lt!(operand_a, operand_b),
        // TODO: are chars considered signed or unsigned?
        TyKind::Char => lt!(operand_a, operand_b),
        TyKind::Float(_) => lt!(operand_a, operand_b),
        TyKind::RawPtr(_) => lt_un!(operand_a, operand_b),
        _ => panic!("Can't eq type  {ty_a:?}"),
    }
}
fn gt_unchecked<'tyctx>(ty_a: Ty<'tyctx>, operand_a: CILNode, operand_b: CILNode) -> CILNode {
    match ty_a.kind() {
        TyKind::Uint(uint) => match uint {
            UintTy::U128 => call!(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_GreaterThan".into(),
                    FnSig::new(&[Type::U128, Type::U128], &Type::Bool),
                    true,
                ),
                [operand_a, operand_b]
            ),
            _ => gt_un!(operand_a, operand_b),
        },
        TyKind::Int(int) => match int {
            IntTy::I128 => call!(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_GreaterThan".into(),
                    FnSig::new(&[Type::I128, Type::I128], &Type::Bool),
                    true,
                ),
                [operand_a, operand_b]
            ),
            _ => gt!(operand_a, operand_b),
        },
        TyKind::Bool => gt!(operand_a, operand_b),
        // TODO: are chars considered signed or unsigned?
        TyKind::Char => gt!(operand_a, operand_b),
        TyKind::Float(_) => gt!(operand_a, operand_b),
        TyKind::RawPtr(_) => gt_un!(operand_a, operand_b),
        _ => panic!("Can't eq type  {ty_a:?}"),
    }
}
fn bit_and_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    operand_a: CILNode,
    operand_b: CILNode,
) -> CILNode {
    let type_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
    match ty_a.kind() {
        TyKind::Uint(UintTy::U128) => call!(
            CallSite::boxed(
                DotnetTypeRef::uint_128().into(),
                "op_BitwiseAnd".into(),
                FnSig::new(&[Type::U128, Type::U128], &Type::U128),
                true,
            ),
            [
                operand_a,
                crate::casts::int_to_int(type_b.clone(), Type::U128, operand_b)
            ]
        ),
        TyKind::Int(IntTy::I128) => call!(
            CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_BitwiseAnd".into(),
                FnSig::new(&[Type::I128, Type::I128], &Type::I128),
                true,
            ),
            [
                operand_a,
                crate::casts::int_to_int(type_b.clone(), Type::I128, operand_b)
            ]
        ),
        _ => and!(operand_a, operand_b),
    }
}
fn bit_or_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    operand_a: CILNode,
    operand_b: CILNode,
) -> CILNode {
    match ty_a.kind() {
        TyKind::Int(IntTy::I128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            call!(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_BitwiseOr".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                ),
                [operand_a, operand_b]
            )
        }
        TyKind::Uint(UintTy::U128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            call!(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_BitwiseOr".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                ),
                [operand_a, operand_b]
            )
        }
        _ => or!(operand_a, operand_b),
    }
}
fn bit_xor_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    todo!()/* 
    match ty_a.kind() {
        TyKind::Int(IntTy::I128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            vec![CILOp::Call(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_ExclusiveOr".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                )
                .into(),
            )]
        }
        TyKind::Uint(UintTy::U128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            vec![CILOp::Call(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_ExclusiveOr".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                )
                .into(),
            )]
        }
        _ => vec![CILOp::XOr],
    }*/
}
fn rem_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    todo!()/* 
    match ty_a.kind() {
        TyKind::Int(IntTy::I128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            vec![CILOp::Call(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_Modulus".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                )
                .into(),
            )]
        }
        TyKind::Uint(UintTy::U128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            vec![CILOp::Call(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_Modulus".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                )
                .into(),
            )]
        }
        _ => vec![CILOp::Rem],
    }*/
}

fn shr_unchecked<'tyctx>(
    value_type: Ty<'tyctx>,
    shift_type: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    todo!()/* 
    let type_b = tycache.type_from_cache(shift_type, tyctx, Some(*method_instance));
    match value_type.kind() {
        TyKind::Uint(UintTy::U128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::uint_128().into(),
                "op_RightShift".into(),
                FnSig::new(&[Type::U128, Type::I32], &Type::U128),
                true,
            )));
            res
        }
        TyKind::Int(IntTy::I128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_RightShift".into(),
                FnSig::new(&[Type::I128, Type::I32], &Type::I128),
                true,
            )));
            res
        }
        TyKind::Uint(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                res.push(CILOp::ShrUn);
                res
            }
            _ => vec![CILOp::ShrUn],
        },
        TyKind::Int(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                res.push(CILOp::Shr);
                res
            }

            _ => vec![CILOp::Shr],
        },
        _ => panic!("Can't bitshift type  {value_type:?}"),
    }*/
}
fn shr_checked<'tyctx>(
    value_type: Ty<'tyctx>,
    shift_type: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    todo!()/* 
    let type_b = tycache.type_from_cache(shift_type, tyctx, Some(*method_instance));
    match value_type.kind() {
        TyKind::Uint(UintTy::U128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::ConvU32(false));
            res.push(CILOp::LdcU32(128));
            res.push(CILOp::RemUn);
            //res.push(CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)));
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::uint_128().into(),
                "op_RightShift".into(),
                FnSig::new(&[Type::U128, Type::I32], &Type::U128),
                true,
            )));
            res
        }
        TyKind::Int(IntTy::I128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::ConvU32(false));
            res.push(CILOp::LdcU32(128));
            res.push(CILOp::RemUn);
            //res.push(CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)));
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_RightShift".into(),
                FnSig::new(&[Type::I128, Type::I32], &Type::I128),
                true,
            )));
            res
        }
        TyKind::Uint(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                res.extend([
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    //CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::ShrUn,
                ]);
                res
            }
            _ => {
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                vec![
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    //CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::ShrUn,
                ]
            }
        },
        TyKind::Int(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                res.extend([
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    //CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::Shr,
                ]);
                res
            }

            _ => {
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                vec![
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    // CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::Shr,
                ]
            }
        },
        _ => panic!("Can't bitshift type  {value_type:?}"),
    }*/
}
fn shl_checked<'tyctx>(
    value_type: Ty<'tyctx>,
    shift_type: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    todo!()/* 
    let type_b = tycache.type_from_cache(shift_type, tyctx, Some(*method_instance));
    match value_type.kind() {
        TyKind::Uint(UintTy::U128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::ConvU32(false));
            res.push(CILOp::LdcU32(128));
            res.push(CILOp::RemUn);
            //res.push(CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)));
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::uint_128().into(),
                "op_LeftShift".into(),
                FnSig::new(&[Type::U128, Type::I32], &Type::U128),
                true,
            )));
            res
        }
        TyKind::Int(IntTy::I128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::ConvU32(false));
            res.push(CILOp::LdcU32(128));
            res.push(CILOp::RemUn);
            //res.push(CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)));
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_LeftShift".into(),
                FnSig::new(&[Type::I128, Type::I32], &Type::I128),
                true,
            )));
            res
        }
        TyKind::Uint(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                res.extend([
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    //CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::Shl,
                ]);
                res
            }
            _ => {
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                vec![
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    //CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::Shl,
                ]
            }
        },
        TyKind::Int(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                res.extend([
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    //CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::Shl,
                ]);
                res
            }

            _ => {
                let bit_cap = (compiletime_sizeof(value_type, tyctx, *method_instance) * 8) as u32;
                vec![
                    CILOp::ConvU32(false),
                    CILOp::LdcU32(bit_cap),
                    CILOp::RemUn,
                    // CILOp::Call(CallSite::boxed(DotnetTypeRef::math().into(),"Abs".into(),FnSig::new(&[Type::I32],&Type::I32),true)),
                    CILOp::Shl,
                ]
            }
        },
        _ => panic!("Can't bitshift type  {value_type:?}"),
    }*/
}
fn shl_unchecked<'tyctx>(
    value_type: Ty<'tyctx>,
    shift_type: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    ops_a: CILNode,
    ops_b: CILNode,
) -> CILNode {
    todo!()/* 
    let type_b = tycache.type_from_cache(shift_type, tyctx, Some(*method_instance));
    match value_type.kind() {
        TyKind::Uint(UintTy::U128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::uint_128().into(),
                "op_LeftShift".into(),
                FnSig::new(&[Type::U128, Type::I32], &Type::U128),
                true,
            )));
            res
        }
        TyKind::Int(IntTy::I128) => {
            let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
            res.push(CILOp::Call(CallSite::boxed(
                DotnetTypeRef::int_128().into(),
                "op_LeftShift".into(),
                FnSig::new(&[Type::I128, Type::I32], &Type::I128),
                true,
            )));
            res
        }
        TyKind::Uint(_) | TyKind::Int(_) => match shift_type.kind() {
            TyKind::Uint(UintTy::U128)
            | TyKind::Int(IntTy::I128)
            | TyKind::Uint(UintTy::U64)
            | TyKind::Int(IntTy::I64) => {
                let mut res = crate::casts::int_to_int(type_b.clone(), Type::I32);
                res.push(CILOp::Shl);
                res
            }
            _ => vec![CILOp::Shl],
        },
        _ => panic!("Can't bitshift type  {value_type:?}"),
    }*/
}
fn mul_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    operand_a: CILNode,
    operand_b: CILNode,
) -> CILNode {
    match ty_a.kind() {
        TyKind::Int(IntTy::I128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            call!(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_Multiply".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                ),
                [operand_a, operand_b]
            )
        }
        TyKind::Uint(UintTy::U128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            call!(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_Multiply".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                ),
                [operand_a, operand_b]
            )
        }
        _ => mul!(operand_a, operand_b),
    }
}
fn div_unchecked<'tyctx>(
    ty_a: Ty<'tyctx>,
    ty_b: Ty<'tyctx>,
    tycache: &mut TyCache,
    method_instance: &Instance<'tyctx>,
    tyctx: TyCtxt<'tyctx>,
    operand_a: CILNode,
    operand_b: CILNode,
) -> CILNode {
    match ty_a.kind() {
        TyKind::Int(IntTy::I128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            call!(
                CallSite::new(
                    Some(DotnetTypeRef::int_128()),
                    "op_Division".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                ),
                [operand_a, operand_b]
            )
        }
        TyKind::Uint(UintTy::U128) => {
            let ty_a = tycache.type_from_cache(ty_a, tyctx, Some(*method_instance));
            let ty_b = tycache.type_from_cache(ty_b, tyctx, Some(*method_instance));
            call!(
                CallSite::new(
                    Some(DotnetTypeRef::uint_128()),
                    "op_Division".into(),
                    FnSig::new(&[ty_a.clone(), ty_b], &ty_a),
                    true,
                ),
                [operand_a, operand_b]
            )
        }
        _ => div!(operand_a, operand_b),
    }
}
