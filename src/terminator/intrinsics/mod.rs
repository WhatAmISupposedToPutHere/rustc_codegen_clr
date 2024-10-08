use crate::{
    assembly::MethodCompileCtx,
    operand::handle_operand,
    place::{place_adress, place_set},
    utilis::field_descrptor,
};
use cilly::{
    call,
    call_site::CallSite,
    call_virt,
    cil_node::CILNode,
    cil_root::CILRoot,
    conv_f32, conv_f64, conv_i16, conv_i32, conv_i64, conv_i8, conv_isize, conv_u16, conv_u32,
    conv_u64, conv_u8, conv_usize, eq, ld_field, ldc_i32, ldc_u32, ldc_u64, size_of, sub,
    v2::{ClassRef, Float, FnSig, Int},
    Type,
};
use ints::{ctlz, rotate_left, rotate_right};
use rustc_middle::{
    mir::{Operand, Place},
    ty::{Instance, ParamEnv, Ty, UintTy},
};
use rustc_span::source_map::Spanned;
use saturating::{saturating_add, saturating_sub};
use type_info::{is_val_statically_known, size_of_val};
use utilis::{
    atomic_add, atomic_and, atomic_max, atomic_min, atomic_nand, atomic_or, atomic_xor,
    compare_bytes,
};
mod bswap;
mod interop;
mod ints;
mod saturating;
mod type_info;
mod utilis;
pub fn handle_intrinsic<'tcx>(
    fn_name: &str,
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    call_instance: Instance<'tcx>,
    span: rustc_span::Span,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
) -> CILRoot {
    match fn_name {
        "arith_offset" => {
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);

            place_set(
                destination,
                handle_operand(&args[0].node, ctx)
                    + handle_operand(&args[1].node, ctx) * conv_isize!(size_of!(tpe)),
                ctx,
            )
        }
        "breakpoint" => {
            debug_assert_eq!(
                args.len(),
                0,
                "The intrinsic `breakpoint` MUST take in exactly 1 argument!"
            );
            CILRoot::Break
        }
        "black_box" => {
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `black_box` MUST take in exactly 1 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            if tpe == Type::Void {
                return CILRoot::Nop;
            }
            // assert_eq!(args.len(),1,"The intrinsic `unlikely` MUST take in exactly 1 argument!");
            place_set(destination, handle_operand(&args[0].node, ctx), ctx)
        }
        "caller_location" => caller_location(destination, ctx, span),
        "compare_bytes" => place_set(
            destination,
            compare_bytes(
                handle_operand(&args[0].node, ctx),
                handle_operand(&args[1].node, ctx),
                handle_operand(&args[2].node, ctx),
                ctx.asm_mut(),
            ),
            ctx,
        ),
        "ctpop" => ints::ctpop(args, destination, call_instance, ctx),
        "bitreverse" => ints::bitreverse(args, destination, ctx, call_instance),
        "ctlz" | "ctlz_nonzero" => ctlz(args, destination, call_instance, ctx),
        "unlikely" | "likely" => {
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `unlikely` MUST take in exactly 1 argument!"
            );
            // assert_eq!(args.len(),1,"The intrinsic `unlikely` MUST take in exactly 1 argument!");
            place_set(destination, handle_operand(&args[0].node, ctx), ctx)
        }
        "is_val_statically_known" => is_val_statically_known(args, destination, ctx),
        "needs_drop" => {
            debug_assert_eq!(
                args.len(),
                0,
                "The intrinsic `needs_drop` MUST take in exactly 0 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let needs_drop = tpe.needs_drop(ctx.tcx(), ParamEnv::reveal_all());
            let needs_drop = i32::from(needs_drop);
            place_set(destination, ldc_i32!(needs_drop), ctx)
        }
        "fmaf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "FusedMultiplyAdd".into(),
                    FnSig::new(
                        [
                            Type::Float(Float::F32),
                            Type::Float(Float::F32),
                            Type::Float(Float::F32)
                        ]
                        .into(),
                        Type::Float(Float::F32)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                    handle_operand(&args[2].node, ctx),
                ]
            ),
            ctx,
        ),
        "fmaf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "FusedMultiplyAdd".into(),
                    FnSig::new(
                        [
                            Type::Float(Float::F64),
                            Type::Float(Float::F64),
                            Type::Float(Float::F64)
                        ]
                        .into(),
                        Type::Float(Float::F64)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                    handle_operand(&args[2].node, ctx),
                ]
            ),
            ctx,
        ),

        "raw_eq" => {
            // Raw eq returns 0 if values are not equal, and 1 if they are, unlike memcmp, which does the oposite.
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            let size = match tpe {
                Type::Bool
                | Type::Int(
                    Int::U8
                    | Int::I8
                    | Int::U16
                    | Int::I16
                    | Int::U32
                    | Int::I32
                    | Int::U64
                    | Int::I64
                    | Int::USize
                    | Int::ISize,
                )
                | Type::Ptr(_) => {
                    return place_set(
                        destination,
                        eq!(
                            handle_operand(&args[0].node, ctx),
                            handle_operand(&args[1].node, ctx)
                        ),
                        ctx,
                    );
                }
                _ => size_of!(tpe),
            };
            place_set(
                destination,
                eq!(
                    compare_bytes(
                        handle_operand(&args[0].node, ctx)
                            .cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::U8))),
                        handle_operand(&args[1].node, ctx)
                            .cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::U8))),
                        conv_usize!(size),
                        ctx.asm_mut()
                    ),
                    ldc_i32!(0)
                ),
                ctx,
            )
        }
        "bswap" => bswap::bswap(args, destination, ctx),
        "cttz" | "cttz_nonzero" => ints::cttz(args, destination, ctx, call_instance),
        "rotate_left" => rotate_left(args, destination, ctx, call_instance),
        "write_bytes" => {
            debug_assert_eq!(
                args.len(),
                3,
                "The intrinsic `write_bytes` MUST take in exactly 3 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            let dst = handle_operand(&args[0].node, ctx);
            let val = handle_operand(&args[1].node, ctx);
            let count = handle_operand(&args[2].node, ctx) * conv_usize!(size_of!(tpe));
            CILRoot::InitBlk {
                dst: Box::new(dst),
                val: Box::new(val),
                count: Box::new(count),
            }
        }
        "copy" => {
            debug_assert_eq!(
                args.len(),
                3,
                "The intrinsic `copy` MUST take in exactly 3 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            let src = handle_operand(&args[0].node, ctx);
            let dst = handle_operand(&args[1].node, ctx);
            let count = handle_operand(&args[2].node, ctx) * conv_usize!(size_of!(tpe));

            CILRoot::CpBlk {
                src: Box::new(src),
                dst: Box::new(dst),
                len: Box::new(count),
            }
        }
        "exact_div" => {
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `exact_div` MUST take in exactly 2 argument!"
            );

            place_set(
                destination,
                crate::binop::binop(
                    rustc_middle::mir::BinOp::Div,
                    &args[0].node,
                    &args[1].node,
                    ctx,
                ),
                ctx,
            )
        }
        "type_id" => {
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            let sig = FnSig::new(
                [ClassRef::runtime_type_hadle(ctx.asm_mut()).into()].into(),
                ClassRef::type_type(ctx.asm_mut()).into(),
            );
            let gethash_sig = FnSig::new(
                [ClassRef::type_type(ctx.asm_mut()).into()].into(),
                Type::Int(Int::I32),
            );
            place_set(
                destination,
                call!(
                    CallSite::boxed(
                        Some(ClassRef::uint_128(ctx.asm_mut())),
                        "op_Implicit".into(),
                        FnSig::new([Type::Int(Int::U32)].into(), Type::Int(Int::U128)),
                        true,
                    ),
                    [conv_u32!(call_virt!(
                        CallSite::boxed(
                            ClassRef::object(ctx.asm_mut()).into(),
                            "GetHashCode".into(),
                            gethash_sig,
                            false,
                        ),
                        [call!(
                            CallSite::boxed(
                                ClassRef::type_type(ctx.asm_mut()).into(),
                                "GetTypeFromHandle".into(),
                                sig,
                                true,
                            ),
                            [CILNode::LDTypeToken(tpe.into())]
                        )]
                    ))]
                ),
                ctx,
            )
        }
        "volatile_load" => volitale_load(args, destination, ctx),
        "volatile_store" => {
            let pointed_type = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let addr_calc = handle_operand(&args[0].node, ctx);
            let value_calc = handle_operand(&args[1].node, ctx);
            CILRoot::Volatile(Box::new(crate::place::ptr_set_op(
                pointed_type.into(),
                ctx,
                addr_calc,
                value_calc,
            )))
        }
        "atomic_load_unordered" => {
            // This is already implemented by default in .NET when volatile is used. TODO: ensure this is 100% right.
            //TODO:fix volitale prefix!
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `atomic_load_unordered` MUST take in exactly 1 argument!"
            );
            let arg = ctx.monomorphize(args[0].node.ty(ctx.body(), ctx.tcx()));
            let arg_ty = arg.builtin_deref(true).unwrap();
            let arg = handle_operand(&args[0].node, ctx);
            let ops = crate::place::deref_op(arg_ty.into(), ctx, arg);
            place_set(destination, ops, ctx)
        }
        "atomic_load_acquire" | "atomic_load_seqcst" => {
            //I am not sure this is implemented propely
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `atomic_load_acquire` MUST take in exactly 1 argument!"
            );
            let ops = handle_operand(&args[0].node, ctx);
            let arg = ctx.monomorphize(args[0].node.ty(ctx.body(), ctx.tcx()));
            let arg_ty = arg.builtin_deref(true).unwrap();

            let ops = crate::place::deref_op(arg_ty.into(), ctx, ops);
            place_set(destination, ops, ctx)
        }
        "atomic_store_relaxed"
        | "atomic_store_seqcst"
        | "atomic_store_release"
        | "atomic_store_unordered" => {
            // This is *propably* wrong :)
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `{fn_name}` MUST take in exactly 1 argument!"
            );
            let addr = handle_operand(&args[0].node, ctx);
            let val = handle_operand(&args[1].node, ctx);
            let arg_ty = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));

            crate::place::ptr_set_op(arg_ty.into(), ctx, addr, val)
        }
        "atomic_cxchgweak_acquire_acquire"
        | "atomic_cxchgweak_acquire_relaxed"
        | "atomic_cxchgweak_relaxed_relaxed"
        | "atomic_cxchgweak_relaxed_acquire"
        | "atomic_cxchgweak_seqcst_acquire"
        | "atomic_cxchgweak_seqcst_seqcst"
        | "atomic_cxchgweak_seqcst_relaxed"
        | "atomic_cxchg_acqrel_acquire"
        | "atomic_cxchg_acquire_seqcst"
        | "atomic_cxchg_release_relaxed"
        | "atomic_cxchg_relaxed_acquire"
        | "atomic_cxchg_acquire_relaxed"
        | "atomic_cxchg_relaxed_seqcst"
        | "atomic_cxchg_acquire_acquire"
        | "atomic_cxchg_release_acquire"
        | "atomic_cxchg_release_seqcst"
        | "atomic_cxchgweak_relaxed_seqcst"
        | "atomic_cxchgweak_acquire_seqcst"
        | "atomic_cxchgweak_release_relaxed"
        | "atomic_cxchgweak_release_acquire"
        | "atomic_cxchgweak_release_seqcst"
        | "atomic_cxchgweak_acqrel_relaxed"
        | "atomic_cxchgweak_acqrel_acquire"
        | "atomic_cxchgweak_acqrel_seqcst"
        | "atomic_cxchg_seqcst_seqcst"
        | "atomic_cxchg_seqcst_acquire"
        | "atomic_cxchg_seqcst_relaxed"
        | "atomic_cxchg_acqrel_relaxed"
        | "atomic_cxchg_relaxed_relaxed"
        | "atomic_cxchg_acqrel_seqcst" => {
            let interlocked = ClassRef::interlocked(ctx.asm_mut());
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let old = handle_operand(&args[1].node, ctx);
            // T
            let src = handle_operand(&args[2].node, ctx);
            debug_assert_eq!(
                args.len(),
                3,
                "The intrinsic `atomic_cxchgweak_acquire_acquire` MUST take in exactly 3 argument!"
            );
            let src_type = ctx.monomorphize(args[2].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            let value = src;
            let comaprand = old.clone();
            #[allow(clippy::single_match_else)]
            let exchange_res = match &src_type {
                Type::Ptr(_) => {
                    let call_site = CallSite::new(
                        Some(interlocked),
                        "CompareExchange".into(),
                        FnSig::new(
                            [
                                ctx.asm_mut().nref(Type::Int(Int::USize)),
                                Type::Int(Int::USize),
                                Type::Int(Int::USize),
                            ]
                            .into(),
                            Type::Int(Int::USize),
                        ),
                        true,
                    );
                    call!(
                        call_site,
                        [
                            Box::new(dst).cast_ptr(ctx.asm_mut().nref(Type::Int(Int::USize))),
                            conv_usize!(value),
                            conv_usize!(comaprand)
                        ]
                    )
                    .cast_ptr(src_type)
                }
                // TODO: this is a bug, on purpose. The 1 byte compare exchange is not supported untill .NET 9. Remove after November, when .NET 9 Releases.
                Type::Int(Int::U8) => comaprand,
                _ => {
                    let call_site = CallSite::new(
                        Some(interlocked),
                        "CompareExchange".into(),
                        FnSig::new(
                            [ctx.asm_mut().nref(src_type), src_type, src_type].into(),
                            src_type,
                        ),
                        true,
                    );
                    call!(call_site, [dst, value, comaprand])
                }
            };

            // Set a field of the destination
            let dst_ty = destination.ty(ctx.body(), ctx.tcx());
            let fld_desc = field_descrptor(dst_ty.ty, 0, ctx);
            assert_eq!(*fld_desc.tpe(), src_type);
            // Set the value of the result.
            let set_val = CILRoot::SetField {
                addr: Box::new(place_adress(destination, ctx)),
                value: Box::new(exchange_res),
                desc: Box::new(fld_desc.clone()),
            };
            // Get the result back
            let val = CILNode::SubTrees(Box::new((
                [set_val].into(),
                ld_field!(place_adress(destination, ctx), fld_desc).into(),
            )));
            // Compare the result to comparand(aka `old`)
            let cmp = eq!(val, old);
            let fld_desc = field_descrptor(dst_ty.ty, 1, ctx);
            assert_eq!(*fld_desc.tpe(), Type::Bool);

            CILRoot::SetField {
                addr: Box::new(place_adress(destination, ctx)),
                value: Box::new(cmp),
                desc: Box::new(fld_desc.clone()),
            }
        }
        "atomic_xsub_release"
        | "atomic_xsub_acqrel"
        | "atomic_xsub_acquire"
        | "atomic_xsub_relaxed"
        | "atomic_xsub_seqcst" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let sub_ammount = handle_operand(&args[1].node, ctx);
            // we sub by adding a negative number
            let add_ammount = CILNode::Neg(Box::new(sub_ammount.clone()));
            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                sub!(
                    atomic_add(dst, add_ammount.clone(), src_type, ctx.asm_mut()),
                    add_ammount
                ),
                ctx,
            )
        }
        "atomic_or_seqcst" | "atomic_or_release" | "atomic_or_acqrel" | "atomic_or_acquire"
        | "atomic_or_relaxed" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let orand = handle_operand(&args[1].node, ctx);

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_or(dst, orand, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_xor_seqcst" | "atomic_xor_release" | "atomic_xor_acqrel" | "atomic_xor_acquire"
        | "atomic_xor_relaxed" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let xorand = handle_operand(&args[1].node, ctx);

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_xor(dst, xorand, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_and_seqcst" | "atomic_and_release" | "atomic_and_acqrel" | "atomic_and_acquire"
        | "atomic_and_relaxed" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let andand = handle_operand(&args[1].node, ctx);

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_and(dst, andand, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_nand_seqcst"
        | "atomic_nand_release"
        | "atomic_nand_acqrel"
        | "atomic_nand_acquire"
        | "atomic_nand_relaxed" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let andand = handle_operand(&args[1].node, ctx);

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_nand(dst, andand, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_fence_acquire"
        | "atomic_fence_seqcst"
        | "atomic_fence_release"
        | "atomic_fence_acqrel" => {
            let thread = ClassRef::thread(ctx.asm_mut());
            CILRoot::Call {
                site: Box::new(CallSite::new(
                    Some(thread),
                    "MemoryBarrier".into(),
                    FnSig::new([].into(), Type::Void),
                    true,
                )),
                args: [].into(),
            }
        }
        "atomic_xadd_release"
        | "atomic_xadd_relaxed"
        | "atomic_xadd_seqcst"
        | "atomic_xadd_acqrel"
        | "atomic_xadd_acquire" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let add_ammount = handle_operand(&args[1].node, ctx);
            // we sub by adding a negative number

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_add(dst, add_ammount, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_umin_release"
        | "atomic_umin_relaxed"
        | "atomic_umin_seqcst"
        | "atomic_umin_acqrel"
        | "atomic_umin_acquire"
        | "atomic_min_release"
        | "atomic_min_relaxed"
        | "atomic_min_seqcst"
        | "atomic_min_acqrel"
        | "atomic_min_acquire" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let min_ammount = handle_operand(&args[1].node, ctx);
            // we sub by mining a negative number

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_min(dst, min_ammount, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_umax_release"
        | "atomic_umax_relaxed"
        | "atomic_umax_seqcst"
        | "atomic_umax_acqrel"
        | "atomic_umax_acquire"
        | "atomic_max_release"
        | "atomic_max_relaxed"
        | "atomic_max_seqcst"
        | "atomic_max_acqrel"
        | "atomic_max_acquire" => {
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let max_ammount = handle_operand(&args[1].node, ctx);
            // we sub by maxing a negative number

            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);

            place_set(
                destination,
                atomic_max(dst, max_ammount, src_type, ctx.asm_mut()),
                ctx,
            )
        }
        "atomic_xchg_release"
        | "atomic_xchg_acquire"
        | "atomic_xchg_acqrel"
        | "atomic_xchg_relaxed"
        | "atomic_xchg_seqcst" => {
            let interlocked = ClassRef::interlocked(ctx.asm_mut());
            // *T
            let dst = handle_operand(&args[0].node, ctx);
            // T
            let new = handle_operand(&args[1].node, ctx);

            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `atomic_xchg_release` MUST take in exactly 3 argument!"
            );
            let src_type = ctx.monomorphize(args[1].node.ty(ctx.body(), ctx.tcx()));
            let src_type = ctx.type_from_cache(src_type);
            match src_type {
                Type::Int(Int::U8) => {
                    return place_set(
                        destination,
                        call!(
                            CallSite::builtin(
                                "atomic_xchng_u8".into(),
                                FnSig::new(
                                    [ctx.asm_mut().nref(Type::Int(Int::U8)), Type::Int(Int::U8)]
                                        .into(),
                                    Type::Int(Int::U8)
                                ),
                                true
                            ),
                            [dst, new]
                        ),
                        ctx,
                    )
                }
                Type::Ptr(_) => {
                    let call_site = CallSite::new(
                        Some(interlocked),
                        "Exchange".into(),
                        FnSig::new(
                            [
                                ctx.asm_mut().nref(Type::Int(Int::USize)),
                                Type::Int(Int::USize),
                            ]
                            .into(),
                            Type::Int(Int::USize),
                        ),
                        true,
                    );
                    return place_set(
                        destination,
                        call!(
                            call_site,
                            [
                                Box::new(dst).cast_ptr(ctx.asm_mut().nref(Type::Int(Int::USize))),
                                conv_usize!(new),
                            ]
                        )
                        .cast_ptr(src_type),
                        ctx,
                    );
                }
                Type::Int(Int::I8 | Int::U16 | Int::I16) | Type::Bool | Type::PlatformChar => {
                    todo!("can't {fn_name} {src_type:?}")
                }
                _ => (),
            }
            let call_site = CallSite::new(
                Some(interlocked),
                "Exchange".into(),
                FnSig::new([ctx.asm_mut().nref(src_type), src_type].into(), src_type),
                true,
            );
            // T
            place_set(destination, call!(call_site, [dst, new]), ctx)
        }
        // TODO:Those are not stricly neccessary, but SHOULD be implemented at some point.
        "assert_inhabited" | "assert_zero_valid" | "const_deallocate" => CILRoot::Nop,
        "ptr_offset_from_unsigned" => {
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `ptr_offset_from_unsigned` MUST take in exactly 1 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            place_set(
                destination,
                CILNode::DivUn(
                    (handle_operand(&args[0].node, ctx) - handle_operand(&args[1].node, ctx))
                        .cast_ptr(Type::Int(Int::USize))
                        .into(),
                    conv_usize!(size_of!(tpe)).into(),
                ),
                ctx,
            )
        }
        "ptr_mask" => {
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `ptr_mask` MUST take in exactly 2 arguments!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            let tpe = ctx.asm_mut().nptr(tpe);

            place_set(
                destination,
                CILNode::And(
                    Box::new(handle_operand(&args[0].node, ctx).cast_ptr(Type::Int(Int::USize))),
                    Box::new(handle_operand(&args[1].node, ctx)),
                )
                .cast_ptr(tpe),
                ctx,
            )
        }
        "ptr_offset_from" => {
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `ptr_offset_from` MUST take in exactly 1 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.type_from_cache(tpe);
            place_set(
                destination,
                CILNode::Div(
                    (handle_operand(&args[0].node, ctx) - handle_operand(&args[1].node, ctx))
                        .cast_ptr(Type::Int(Int::ISize))
                        .into(),
                    conv_isize!(size_of!(tpe)).into(),
                ),
                ctx,
            )
        }
        "saturating_add" => saturating_add(args, destination, ctx, call_instance),
        "saturating_sub" => saturating_sub(args, destination, ctx, call_instance),
        "min_align_of_val" => {
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `min_align_of_val` MUST take in exactly 1 argument!"
            );
            let tpe = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            place_set(
                destination,
                conv_usize!(ldc_u64!(crate::utilis::align_of(tpe, ctx.tcx()))),
                ctx,
            )
        }
        // .NET guarantess all loads are tear-free
        "atomic_load_relaxed" => {
            //I am not sure this is implemented propely
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `atomic_load_relaxed` MUST take in exactly 1 argument!"
            );
            let ops = handle_operand(&args[0].node, ctx);
            let arg = ctx.monomorphize(args[0].node.ty(ctx.body(), ctx.tcx()));
            let arg_ty = arg.builtin_deref(true).unwrap();

            let ops = crate::place::deref_op(arg_ty.into(), ctx, ops);
            place_set(destination, ops, ctx)
        }
        "sqrtf32" => {
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `sqrtf32` MUST take in exactly 1 argument!"
            );
            place_set(
                destination,
                call!(
                    CallSite::boxed(
                        Some(ClassRef::mathf(ctx.asm_mut())),
                        "Sqrt".into(),
                        FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                        true,
                    ),
                    [handle_operand(&args[0].node, ctx)]
                ),
                ctx,
            )
        }

        "powif32" => {
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `powif32` MUST take in exactly 2 arguments!"
            );

            place_set(
                destination,
                call!(
                    CallSite::boxed(
                        Some(ClassRef::single(ctx.asm_mut())),
                        "Pow".into(),
                        FnSig::new(
                            [Type::Float(Float::F32), Type::Float(Float::F32)].into(),
                            Type::Float(Float::F32)
                        ),
                        true,
                    ),
                    [
                        handle_operand(&args[0].node, ctx),
                        conv_f32!(handle_operand(&args[1].node, ctx))
                    ]
                ),
                ctx,
            )
        }
        "powif64" => {
            debug_assert_eq!(
                args.len(),
                2,
                "The intrinsic `powif64` MUST take in exactly 2 arguments!"
            );

            place_set(
                destination,
                call!(
                    CallSite::boxed(
                        Some(ClassRef::double(ctx.asm_mut())),
                        "Pow".into(),
                        FnSig::new(
                            [Type::Float(Float::F64), Type::Float(Float::F64)].into(),
                            Type::Float(Float::F64)
                        ),
                        true,
                    ),
                    [
                        handle_operand(&args[0].node, ctx),
                        conv_f64!(handle_operand(&args[1].node, ctx))
                    ]
                ),
                ctx,
            )
        }
        "size_of_val" => size_of_val(args, destination, ctx, call_instance),
        "typed_swap" => {
            let pointed_ty = ctx.monomorphize(
                call_instance.args[0]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.monomorphize(pointed_ty);
            let tpe = ctx.type_from_cache(tpe);
            CILRoot::Call {
                site: Box::new(CallSite::builtin(
                    "swap_at_generic".into(),
                    FnSig::new(
                        [
                            ctx.asm_mut().nptr(Type::Void),
                            ctx.asm_mut().nptr(Type::Void),
                            Type::Int(Int::USize),
                        ]
                        .into(),
                        Type::Void,
                    ),
                    true,
                )),
                args: [
                    handle_operand(&args[0].node, ctx).cast_ptr(ctx.asm_mut().nptr(Type::Void)),
                    handle_operand(&args[1].node, ctx).cast_ptr(ctx.asm_mut().nptr(Type::Void)),
                    conv_usize!(size_of!(tpe)),
                ]
                .into(),
            }
        }

        "type_name" => {
            let const_val = ctx
                .tcx()
                .const_eval_instance(ParamEnv::reveal_all(), call_instance, span)
                .unwrap();
            place_set(
                destination,
                crate::constant::load_const_value(const_val, Ty::new_static_str(ctx.tcx()), ctx),
                ctx,
            )
        }
        "float_to_int_unchecked" => {
            let tpe = ctx.monomorphize(
                call_instance.args[1]
                    .as_type()
                    .expect("needs_drop works only on types!"),
            );
            let tpe = ctx.monomorphize(tpe);
            let tpe = ctx.type_from_cache(tpe);
            let input = handle_operand(&args[0].node, ctx);
            place_set(
                destination,
                match tpe {
                    Type::Int(Int::U8) => conv_u8!(input),
                    Type::Int(Int::U16) => conv_u16!(input),
                    Type::Int(Int::U32) => conv_u32!(input),
                    Type::Int(Int::U64) => conv_u64!(input),
                    Type::Int(Int::USize) => conv_usize!(input),
                    Type::Int(Int::I8) => conv_i8!(input),
                    Type::Int(Int::I16) => conv_i16!(input),
                    Type::Int(Int::I32) => conv_i32!(input),
                    Type::Int(Int::I64) => conv_i64!(input),
                    Type::Int(Int::ISize) => conv_isize!(input),
                    _ => todo!("can't float_to_int_unchecked on {tpe:?}"),
                },
                ctx,
            )
        }
        "fabsf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Abs".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "fabsf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Abs".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "expf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Exp".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "expf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Exp".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "logf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Log".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "logf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Log".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "log2f32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Log2".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "log2f64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Log2".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "log10f32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Log10".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "log10f64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Log10".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "powf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Pow".into(),
                    FnSig::new(
                        [Type::Float(Float::F32), Type::Float(Float::F32)].into(),
                        Type::Float(Float::F32)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "powf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Pow".into(),
                    FnSig::new(
                        [Type::Float(Float::F64), Type::Float(Float::F64)].into(),
                        Type::Float(Float::F64)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "copysignf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "CopySign".into(),
                    FnSig::new(
                        [Type::Float(Float::F32), Type::Float(Float::F32)].into(),
                        Type::Float(Float::F32)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "copysignf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "CopySign".into(),
                    FnSig::new(
                        [Type::Float(Float::F64), Type::Float(Float::F64)].into(),
                        Type::Float(Float::F64)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "sinf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Sin".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "sinf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Sin".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "cosf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Cos".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "cosf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Cos".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "exp2f32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "Exp2".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "exp2f64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "Exp2".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "truncf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::mathf(ctx.asm_mut()),
                    "Truncate".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "truncf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::math(ctx.asm_mut()),
                    "Truncate".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        // `roundf32` should be a differnt intrinsics, but it requires some .NET fuckery to implement(.NET enums are **wierd**)
        "nearbyintf32" | "rintf32" | "roundevenf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::mathf(ctx.asm_mut()),
                    "Round".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "roundf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::mathf(ctx.asm_mut()),
                    "Round".into(),
                    FnSig::new(
                        [
                            Type::Float(Float::F32),
                            Type::ClassRef(ClassRef::midpoint_rounding(ctx.asm_mut())),
                        ]
                        .into(),
                        Type::Float(Float::F32)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    ldc_i32!(1).transmute_on_stack(
                        Type::Int(Int::I32),
                        Type::ClassRef(ClassRef::midpoint_rounding(ctx.asm_mut())),
                        ctx.asm_mut()
                    )
                ]
            ),
            ctx,
        ),
        "nearbyintf64" | "rintf64" | "roundevenf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::math(ctx.asm_mut()),
                    "Round".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "roundf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::math(ctx.asm_mut()),
                    "Round".into(),
                    FnSig::new(
                        [
                            Type::Float(Float::F64),
                            Type::ClassRef(ClassRef::midpoint_rounding(ctx.asm_mut()))
                        ]
                        .into(),
                        Type::Float(Float::F64)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    ldc_i32!(1).transmute_on_stack(
                        Type::Int(Int::I32),
                        Type::ClassRef(ClassRef::midpoint_rounding(ctx.asm_mut())),
                        ctx.asm_mut()
                    )
                ]
            ),
            ctx,
        ),
        "floorf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::mathf(ctx.asm_mut()),
                    "Floor".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "floorf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::math(ctx.asm_mut()),
                    "Floor".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "ceilf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::mathf(ctx.asm_mut()),
                    "Ceiling".into(),
                    FnSig::new([Type::Float(Float::F32)].into(), Type::Float(Float::F32)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "ceilf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::math(ctx.asm_mut()),
                    "Ceiling".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true
                ),
                [handle_operand(&args[0].node, ctx),]
            ),
            ctx,
        ),
        "maxnumf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "MaxNumber".into(),
                    FnSig::new(
                        [Type::Float(Float::F64), Type::Float(Float::F64)].into(),
                        Type::Float(Float::F64)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "maxnumf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "MaxNumber".into(),
                    FnSig::new(
                        [Type::Float(Float::F32), Type::Float(Float::F32)].into(),
                        Type::Float(Float::F32)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "minnumf64" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::double(ctx.asm_mut()),
                    "MinNumber".into(),
                    FnSig::new(
                        [Type::Float(Float::F64), Type::Float(Float::F64)].into(),
                        Type::Float(Float::F64)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "minnumf32" => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::single(ctx.asm_mut()),
                    "MinNumber".into(),
                    FnSig::new(
                        [Type::Float(Float::F32), Type::Float(Float::F32)].into(),
                        Type::Float(Float::F32)
                    ),
                    true
                ),
                [
                    handle_operand(&args[0].node, ctx),
                    handle_operand(&args[1].node, ctx),
                ]
            ),
            ctx,
        ),
        "variant_count" => {
            let const_val = ctx
                .tcx()
                .const_eval_instance(ParamEnv::reveal_all(), call_instance, span)
                .unwrap();
            place_set(
                destination,
                crate::constant::load_const_value(
                    const_val,
                    Ty::new_uint(ctx.tcx(), UintTy::Usize),
                    ctx,
                ),
                ctx,
            )
        }
        "sqrtf64" => {
            debug_assert_eq!(
                args.len(),
                1,
                "The intrinsic `sqrtf64` MUST take in exactly 1 argument!"
            );

            let ops = call!(
                CallSite::boxed(
                    Some(ClassRef::math(ctx.asm_mut())),
                    "Sqrt".into(),
                    FnSig::new([Type::Float(Float::F64)].into(), Type::Float(Float::F64)),
                    true,
                ),
                [handle_operand(&args[0].node, ctx)]
            );
            place_set(destination, ops, ctx)
        }
        "rotate_right" => rotate_right(args, destination, ctx, call_instance),
        "catch_unwind" => {
            debug_assert_eq!(
                args.len(),
                3,
                "The intrinsic `catch_unwind` MUST take in exactly 3 arguments!"
            );
            let try_fn = handle_operand(&args[0].node, ctx);
            let data_ptr = handle_operand(&args[1].node, ctx);
            let catch_fn = handle_operand(&args[2].node, ctx);
            let uint8_ptr = ctx.asm_mut().nptr(Type::Int(Int::U8));
            place_set(
                destination,
                call!(
                    CallSite::builtin(
                        "catch_unwind".into(),
                        FnSig::new(
                            [
                                Type::FnPtr(ctx.asm_mut().sig([uint8_ptr], Type::Void)),
                                uint8_ptr,
                                Type::FnPtr(ctx.asm_mut().sig([uint8_ptr, uint8_ptr], Type::Void)),
                            ]
                            .into(),
                            Type::Int(Int::I32),
                        ),
                        true
                    ),
                    [try_fn, data_ptr, catch_fn]
                ),
                ctx,
            )
        }
        "abort" => CILRoot::throw("Called abort!", ctx.asm_mut()),
        "const_allocate" => place_set(destination, conv_usize!(ldc_u32!(0)), ctx),
        "vtable_size" => {
            let vtableptr = handle_operand(&args[0].node, ctx);
            place_set(
                destination,
                CILNode::LDIndUSize {
                    ptr: Box::new(
                        (vtableptr + conv_usize!((size_of!(Type::Int(Int::ISize)))))
                            .cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::USize))),
                    ),
                },
                ctx,
            )
        }
        "vtable_align" => {
            let vtableptr = handle_operand(&args[0].node, ctx);
            place_set(
                destination,
                CILNode::LDIndUSize {
                    ptr: Box::new(
                        (vtableptr + conv_usize!((size_of!(Type::Int(Int::ISize))) * ldc_i32!(2)))
                            .cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::USize))),
                    ),
                },
                ctx,
            )
        }
        _ => intrinsic_slow(fn_name, args, destination, ctx, call_instance, span),
    }
}
fn intrinsic_slow<'tcx>(
    fn_name: &str,
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    call_instance: Instance<'tcx>,
    span: rustc_span::Span,
) -> CILRoot {
    let _ = span;

    if fn_name.contains("likely") {
        debug_assert_eq!(
            args.len(),
            1,
            "The intrinsic `fn_name` MUST take in exactly 1 argument!"
        );
        // assert_eq!(args.len(),1,"The intrinsic `unlikely` MUST take in exactly 1 argument!");
        place_set(destination, handle_operand(&args[0].node, ctx), ctx)
    } else if fn_name.contains("volitale_load") {
        volitale_load(args, destination, ctx)
    } else if fn_name.contains("type_id") {
        let tpe = ctx.monomorphize(
            call_instance.args[0]
                .as_type()
                .expect("needs_drop works only on types!"),
        );
        let tpe = ctx.type_from_cache(tpe);
        let sig = FnSig::new(
            [ClassRef::runtime_type_hadle(ctx.asm_mut()).into()].into(),
            ClassRef::type_type(ctx.asm_mut()).into(),
        );
        let gethash_sig = FnSig::new(
            [ClassRef::type_type(ctx.asm_mut()).into()].into(),
            Type::Int(Int::I32),
        );
        place_set(
            destination,
            call!(
                CallSite::boxed(
                    Some(ClassRef::uint_128(ctx.asm_mut())),
                    "op_Implicit".into(),
                    FnSig::new([Type::Int(Int::U32)].into(), Type::Int(Int::U128)),
                    true,
                ),
                [conv_u32!(call_virt!(
                    CallSite::boxed(
                        ClassRef::object(ctx.asm_mut()).into(),
                        "GetHashCode".into(),
                        gethash_sig,
                        false,
                    ),
                    [call!(
                        CallSite::boxed(
                            ClassRef::type_type(ctx.asm_mut()).into(),
                            "GetTypeFromHandle".into(),
                            sig,
                            true,
                        ),
                        [CILNode::LDTypeToken(tpe.into())]
                    )]
                ))]
            ),
            ctx,
        )
    } else if fn_name.contains("size_of_val") {
        size_of_val(args, destination, ctx, call_instance)
    } else if fn_name.contains("is_val_statically_known") {
        is_val_statically_known(args, destination, ctx)
    } else if fn_name.contains("min_align_of_val") {
        debug_assert_eq!(
            args.len(),
            1,
            "The intrinsic `min_align_of_val` MUST take in exactly 1 argument!"
        );
        let tpe = ctx.monomorphize(
            call_instance.args[0]
                .as_type()
                .expect("needs_drop works only on types!"),
        );
        place_set(
            destination,
            conv_usize!(ldc_u64!(crate::utilis::align_of(tpe, ctx.tcx()))),
            ctx,
        )
    } else if fn_name.contains("typed_swap") {
        let pointed_ty = ctx.monomorphize(
            call_instance.args[0]
                .as_type()
                .expect("needs_drop works only on types!"),
        );
        let tpe = ctx.monomorphize(pointed_ty);
        let tpe = ctx.type_from_cache(tpe);
        CILRoot::Call {
            site: Box::new(CallSite::builtin(
                "swap_at_generic".into(),
                FnSig::new(
                    [
                        ctx.asm_mut().nptr(Type::Void),
                        ctx.asm_mut().nptr(Type::Void),
                        Type::Int(Int::USize),
                    ]
                    .into(),
                    Type::Void,
                ),
                true,
            )),
            args: [
                handle_operand(&args[0].node, ctx).cast_ptr(ctx.asm_mut().nptr(Type::Void)),
                handle_operand(&args[1].node, ctx).cast_ptr(ctx.asm_mut().nptr(Type::Void)),
                conv_usize!(size_of!(tpe)),
            ]
            .into(),
        }
    } else if fn_name.contains("type_name") {
        let const_val = ctx
            .tcx()
            .const_eval_instance(ParamEnv::reveal_all(), call_instance, span)
            .unwrap();
        place_set(
            destination,
            crate::constant::load_const_value(const_val, Ty::new_static_str(ctx.tcx()), ctx),
            ctx,
        )
    } else if fn_name.contains("select_unpredictable") {
        let select_ty = ctx.monomorphize(
            call_instance.args[0]
                .as_type()
                .expect("needs_drop works only on types!"),
        );
        let select_ty = ctx.monomorphize(select_ty);
        let select_ty = ctx.type_from_cache(select_ty);
        place_set(
            destination,
            CILNode::select(
                select_ty,
                handle_operand(&args[1].node, ctx),
                handle_operand(&args[2].node, ctx),
                handle_operand(&args[0].node, ctx),
            ),
            ctx,
        )
    } else if fn_name.contains("const_allocate") {
        place_set(destination, conv_usize!(ldc_u32!(0)), ctx)
    } else if fn_name.contains("const_deallocate") {
        CILRoot::Nop
    } else if fn_name.contains("vtable_size") {
        let vtableptr = handle_operand(&args[0].node, ctx);
        place_set(
            destination,
            CILNode::LDIndUSize {
                ptr: Box::new(
                    (vtableptr + conv_usize!((size_of!(Type::Int(Int::ISize)))))
                        .cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::USize))),
                ),
            },
            ctx,
        )
    } else if fn_name.contains("vtable_align") {
        let vtableptr = handle_operand(&args[0].node, ctx);
        place_set(
            destination,
            CILNode::LDIndUSize {
                ptr: Box::new(
                    (vtableptr + conv_usize!((size_of!(Type::Int(Int::ISize))) * ldc_i32!(2)))
                        .cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::USize))),
                ),
            },
            ctx,
        )
    } else if fn_name.contains("ptr_guaranteed_cmp") {
        let lhs = handle_operand(&args[0].node, ctx);
        let rhs = handle_operand(&args[0].node, ctx);
        place_set(destination, conv_u8!(eq!(lhs, rhs)), ctx)
    } else {
        todo!("Unhandled intrinsic {fn_name}.")
    }
}
fn volitale_load<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
) -> CILRoot {
    //TODO:fix volitale prefix!
    debug_assert_eq!(
        args.len(),
        1,
        "The intrinsic `volatile_load` MUST take in exactly 1 argument!"
    );
    let arg = ctx.monomorphize(args[0].node.ty(ctx.body(), ctx.tcx()));
    let arg_ty = arg.builtin_deref(true).unwrap();
    let arg = handle_operand(&args[0].node, ctx);
    let ops = CILNode::Volatile(Box::new(crate::place::deref_op(arg_ty.into(), ctx, arg)));
    place_set(destination, ops, ctx)
}
fn caller_location<'tcx>(
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    span: rustc_span::Span,
) -> CILRoot {
    let caller_loc = ctx.tcx().span_as_caller_location(span);
    let caller_loc_ty = ctx.tcx().caller_location_ty();
    crate::place::place_set(
        destination,
        crate::constant::load_const_value(caller_loc, caller_loc_ty, ctx),
        ctx,
    )
}
