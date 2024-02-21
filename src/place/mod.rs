// FIXME: This file may contain unnecesary morphize calls.

use crate::cil::CILOp;
use crate::cil_tree::cil_node::CILNode;
use crate::r#type::{pointer_to_is_fat, DotnetTypeRef, Type};

use rustc_middle::mir::Place;

mod adress;
mod body;
mod get;
mod set;
pub use adress::*;
pub use body::*;
pub use get::*;
use rustc_middle::ty::{FloatTy, Instance, IntTy, ParamEnv, Ty, TyCtxt, TyKind, UintTy};
pub use set::*;
fn slice_head<T>(slice: &[T]) -> (&T, &[T]) {
    assert!(!slice.is_empty());
    let last = &slice[slice.len() - 1];
    (last, &slice[..(slice.len() - 1)])
}
fn pointed_type(ty: PlaceTy) -> Ty {
    if let PlaceTy::Ty(ty) = ty {
        if let TyKind::Ref(_region, inner, _mut) = ty.kind() {
            *inner
        } else if let TyKind::RawPtr(inner_and_mut) = ty.kind() {
            inner_and_mut.ty
        } else {
            panic!("{ty:?} is not a pointer type!");
        }
    } else {
        panic!("Can't dereference enum variant!");
    }
}
fn body_ty_is_by_adress(last_ty: Ty) -> bool {
    crate::assert_morphic!(last_ty);
    match *last_ty.kind() {
        TyKind::Adt(_, _) | TyKind::Closure(_, _) | TyKind::Array(_, _) => true,
        // True for non-0 tuples
        TyKind::Tuple(elements) => !elements.is_empty(),

        //TODO: check if slices are handled propely
        TyKind::Slice(_) | TyKind::Str => true,

        TyKind::Int(_)
        | TyKind::Float(_)
        | TyKind::Uint(_)
        | TyKind::Ref(_, _, _)
        | TyKind::RawPtr(_)
        | TyKind::Bool
        | TyKind::Char => false,

        _ => todo!(
            "TODO: body_ty_is_by_adress does not support type {last_ty:?} kind:{kind:?}",
            kind = last_ty.kind()
        ),
    }
}

fn place_get_length<'ctx>(
    curr_type: PlaceTy<'ctx>,
    tyctx: TyCtxt<'ctx>,
    method_instance: Instance<'ctx>,
    type_cache: &mut crate::r#type::TyCache,
) -> Vec<CILOp> {
    let curr_ty = curr_type.as_ty().expect("Can't index into enum!");
    let curr_ty = crate::utilis::monomorphize(&method_instance, curr_ty, tyctx);
    let tpe = type_cache.type_from_cache(curr_ty, tyctx, Some(method_instance));
    let Type::DotnetType(class) = &tpe else {
        panic!("Can't index into type {tpe:?}");
    };

    match *curr_ty.kind() {
        TyKind::Array(_elem, len) => {
            let len = crate::utilis::monomorphize(&method_instance, len, tyctx);
            let len = len.eval_target_usize(tyctx, ParamEnv::reveal_all()) as i64;
            vec![CILOp::LdcI64(len)]
        }
        TyKind::Slice(_elem) => {
            let signature = crate::function_sig::FnSig::new(&[tpe.clone()], &Type::USize);
            vec![CILOp::Call(crate::cil::CallSite::boxed(
                Some(class.as_ref().clone()),
                "get_Length".into(),
                signature,
                false,
            ))]
        }
        _ => todo!("Can't get length of non-array/slice!"),
    }
}

/// Given a type `derefed_type`, it retuns a set of instructions to get a value behind a pointer to `derefed_type`.
pub fn deref_op<'ctx>(
    derefed_type: PlaceTy<'ctx>,
    tyctx: TyCtxt<'ctx>,
    method_instance: &Instance<'ctx>,
    type_cache: &mut crate::r#type::TyCache,
    ptr: CILNode,
) -> CILNode {
    let ptr = Box::new(ptr);
    let res = if let PlaceTy::Ty(derefed_type) = derefed_type {
        match derefed_type.kind() {
            TyKind::Int(int_ty) => match int_ty {
                IntTy::I8 => CILNode::LDIndI8 { ptr },
                IntTy::I16 => CILNode::LDIndI16 { ptr },
                IntTy::I32 => CILNode::LDIndI32 { ptr },
                IntTy::I64 => CILNode::LDIndI64 { ptr },
                IntTy::Isize => CILNode::LDIndISize { ptr },
                IntTy::I128 => CILNode::LdObj {
                    ptr,
                    obj: Box::new(DotnetTypeRef::int_128().into()),
                },
                //_ => todo!("TODO: can't deref int type {int_ty:?} yet"),
            },
            TyKind::Uint(int_ty) => match int_ty {
                UintTy::U8 => CILNode::LDIndI8 { ptr },
                UintTy::U16 => CILNode::LDIndI16 { ptr },
                UintTy::U32 => CILNode::LDIndI32 { ptr },
                UintTy::U64 => CILNode::LDIndI64 { ptr },
                UintTy::Usize => CILNode::LDIndISize { ptr },
                UintTy::U128 => CILNode::LdObj {
                    ptr,
                    obj: Box::new(DotnetTypeRef::uint_128().into()),
                }, //vec![CILOp::LdObj(Box::new())],
                   //_ => todo!("TODO: can't deref int type {int_ty:?} yet"),
            },
            TyKind::Float(float_ty) => match float_ty {
                FloatTy::F32 => CILNode::LDIndF32 { ptr },
                FloatTy::F64 => CILNode::LDIndF64 { ptr },
            },
            TyKind::Bool => CILNode::LDIndI8 { ptr }, // Both Rust bool and a managed bool are 1 byte wide. .NET bools are 4 byte wide only in the context of Marshaling/PInvoke,
            // due to historic reasons(BOOL was an alias for int in early Windows, and it stayed this way.) - FractalFir
            TyKind::Char => CILNode::LDIndI32 { ptr }, // always 4 bytes wide: https://doc.rust-lang.org/std/primitive.char.html#representation
            TyKind::Adt(_, _) | TyKind::Tuple(_) => {
                let derefed_type =
                    type_cache.type_from_cache(derefed_type, tyctx, Some(*method_instance));

                CILNode::LdObj {
                    ptr,
                    obj: Box::new(derefed_type),
                }
            }
            TyKind::Ref(_, inner, _) => {
                if pointer_to_is_fat(*inner, tyctx, Some(*method_instance)) {
                    CILNode::LdObj {
                        ptr,
                        obj: Box::new(
                            type_cache
                                .type_from_cache(derefed_type, tyctx, Some(*method_instance))
                                .into(),
                        ),
                    }
                } else {
                    CILNode::LDIndISize { ptr }
                }
            }
            TyKind::RawPtr(type_and_mut) => {
                if pointer_to_is_fat(type_and_mut.ty, tyctx, Some(*method_instance)) {
                    CILNode::LdObj {
                        ptr,
                        obj: Box::new(
                            type_cache
                                .type_from_cache(derefed_type, tyctx, Some(*method_instance))
                                .into(),
                        ),
                    }
                } else {
                    CILNode::LDIndISize { ptr }
                }
            }
            TyKind::Array(_, _) => {
                let derefed_type =
                    type_cache.type_from_cache(derefed_type, tyctx, Some(*method_instance));
                CILNode::LdObj {
                    ptr,
                    obj: Box::new(derefed_type),
                }
            }
            TyKind::FnPtr(_) => {
                let derefed_type =
                    type_cache.type_from_cache(derefed_type, tyctx, Some(*method_instance));
                CILNode::LdObj {
                    ptr,
                    obj: Box::new(derefed_type),
                }
            }
            TyKind::Closure(_, _) => {
                let derefed_type =
                    type_cache.type_from_cache(derefed_type, tyctx, Some(*method_instance));
                CILNode::LdObj {
                    ptr,
                    obj: Box::new(derefed_type),
                }
            }
            _ => todo!("TODO: can't deref type {derefed_type:?} yet"),
        }
    } else {
        todo!("Can't dereference enum variants yet!")
    };
    res
}

/// Returns the ops for getting the value of  a given place.
pub fn place_adress<'a>(
    place: &Place<'a>,
    ctx: TyCtxt<'a>,
    method: &rustc_middle::mir::Body<'a>,
    method_instance: Instance<'a>,
    type_cache: &mut crate::r#type::TyCache,
) -> CILNode {
    let place_ty = place.ty(method, ctx);
    let place_ty = crate::utilis::monomorphize(&method_instance, place_ty, ctx).ty;
    if place.projection.is_empty() {
        local_adress(place.local.as_usize(), method)
    } else {
        let (mut addr_calc, mut ty) = local_body(place.local.as_usize(), method);

        ty = crate::utilis::monomorphize(&method_instance, ty, ctx);
        let mut ty = ty.into();

        let (head, body) = slice_head(place.projection);
        for elem in body {
            let (curr_ty, curr_ops) = place_elem_body(
                elem,
                ty,
                ctx,
                method_instance,
                method,
                type_cache,
                addr_calc.clone(),
            );
            ty = curr_ty.monomorphize(&method_instance, ctx);
            addr_calc = curr_ops;
        }
        let mut ops = addr_calc.flatten();
        adress::place_elem_adress(
            head,
            ty,
            ctx,
            method_instance,
            method,
            type_cache,
            place_ty,
            addr_calc,
        )
    }
}
pub(crate) fn place_set<'a>(
    place: &Place<'a>,
    ctx: TyCtxt<'a>,
    value_calc: Vec<CILOp>,
    method: &rustc_middle::mir::Body<'a>,
    method_instance: Instance<'a>,
    type_cache: &mut crate::r#type::TyCache,
) -> Vec<CILOp> {
    if place.projection.is_empty() {
        let mut ops = Vec::with_capacity(place.projection.len());
        ops.extend(value_calc);
        ops.push(set::local_set(place.local.as_usize(), method));
        ops
    } else {
        let mut addr_calc = local_body(place.local.as_usize(), method);
        let (op, ty) = local_body(place.local.as_usize(), method);

        let mut ty: PlaceTy = ty.into();
        ty = ty.monomorphize(&method_instance, ctx);

        let (head, body) = slice_head(place.projection);
        for elem in body {
            let (curr_ty, curr_ops) = place_elem_body(
                elem,
                ty,
                ctx,
                method_instance,
                method,
                type_cache,
                addr_calc.0,
            );
            ty = curr_ty.monomorphize(&method_instance, ctx);
            addr_calc.0 = curr_ops;
        }
        //
        ty = ty.monomorphize(&method_instance, ctx);
        place_elem_set(
            head,
            ty,
            ctx,
            method_instance,
            type_cache,
            value_calc,
            addr_calc.0.flatten(),
        )
    }
}
#[derive(Debug, Clone, Copy)]
pub enum PlaceTy<'ctx> {
    Ty(Ty<'ctx>),
    EnumVariant(Ty<'ctx>, u32),
}
impl<'ctx> From<Ty<'ctx>> for PlaceTy<'ctx> {
    fn from(ty: Ty<'ctx>) -> Self {
        Self::Ty(ty)
    }
}
impl<'ctx> PlaceTy<'ctx> {
    pub fn monomorphize(&self, method_instance: &Instance<'ctx>, ctx: TyCtxt<'ctx>) -> Self {
        match self {
            Self::Ty(inner) => Self::Ty(crate::utilis::monomorphize(method_instance, *inner, ctx)),
            Self::EnumVariant(enm, variant) => Self::EnumVariant(
                crate::utilis::monomorphize(method_instance, *enm, ctx),
                *variant,
            ),
        }
    }
    pub fn as_ty(&self) -> Option<Ty<'ctx>> {
        match self {
            Self::Ty(inner) => Some(*inner),
            Self::EnumVariant(..) => None,
        }
    }
    /// Returns the kind of the underlyting Ty.
    pub fn kind(&self) -> &TyKind<'ctx> {
        match self {
            Self::Ty(ty) => ty.kind(),
            //TODO: find a better way to get the emum variant!
            Self::EnumVariant(ty, _variant) => ty.kind(),
        }
    }
}
