use crate::{
    assembly::MethodCompileCtx,
    call_info::CallInfo,
    operand::handle_operand,
    r#type::{fat_ptr_to, get_type, pointer_to_is_fat},
};
use cilly::{
    call_site::CallSite,
    cil_node::CILNode,
    cil_root::CILRoot,
    conv_usize,
    field_desc::FieldDescriptor,
    fn_sig::FnSig,
    ld_field, ldc_i32, ldc_u64, size_of,
    v2::{Float, Int},
    Type,
};
use rustc_middle::{
    mir::{CastKind, NullOp, Operand, Place, Rvalue},
    ty::{adjustment::PointerCoercion, GenericArgs, Instance, InstanceKind, ParamEnv, Ty, TyKind},
};
macro_rules! cast {
    ($ctx:ident,$operand:ident,$target:ident,$cast_name:path,$asm:expr) => {{
        let target = $ctx.monomorphize(*$target);
        let target = $ctx.type_from_cache(target);
        let src = $operand.ty(&$ctx.body().local_decls, $ctx.tcx());
        let src = $ctx.monomorphize(src);
        let src = $ctx.type_from_cache(src);
        $cast_name(src, &target, handle_operand($operand, $ctx), $asm)
    }};
}
pub fn is_rvalue_unint<'tcx>(rvalue: &Rvalue<'tcx>, ctx: &mut MethodCompileCtx<'tcx, '_>) -> bool {
    match rvalue {
        Rvalue::Repeat(operand, _) | Rvalue::Use(operand) => {
            crate::operand::is_uninit(operand, ctx)
        }
        /* TODO: before enabling this, check if the aggregate is an enum, and if so, check if it has a discriminant.
        Rvalue::Aggregate(_, field_index) => field_index
        .iter()
        .all(|operand| crate::operand::is_uninit(operand, ctx)),*/
        _ => false,
    }
}
pub fn handle_rvalue<'tcx>(
    rvalue: &Rvalue<'tcx>,
    target_location: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
) -> CILNode {
    match rvalue {
        Rvalue::Use(operand) => handle_operand(operand, ctx),
        Rvalue::CopyForDeref(place) => crate::place::place_get(place, ctx),

        Rvalue::Ref(_region, _borrow_kind, place) => crate::place::place_adress(place, ctx),
        Rvalue::RawPtr(_mutability, place) => crate::place::place_adress(place, ctx),
        Rvalue::Cast(
            CastKind::PointerCoercion(PointerCoercion::UnsafeFnPointer),
            operand,
            _dst,
        ) => handle_operand(operand, ctx),
        Rvalue::Cast(
            CastKind::PointerCoercion(
                PointerCoercion::MutToConstPointer | PointerCoercion::ArrayToPointer,
            )
            | CastKind::PtrToPtr,
            operand,
            dst,
        ) => ptr_to_ptr(ctx, operand, *dst),
        Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize), operand, target) => {
            crate::unsize::unsize2(ctx, operand, *target)
        }
        Rvalue::BinaryOp(binop, operands) => {
            crate::binop::binop(*binop, &operands.0, &operands.1, ctx)
        }
        Rvalue::UnaryOp(binop, operand) => crate::unop::unop(*binop, operand, ctx),
        Rvalue::Cast(CastKind::IntToInt, operand, target) => {
            cast!(
                ctx,
                operand,
                target,
                crate::casts::int_to_int,
                ctx.asm_mut()
            )
        }
        Rvalue::Cast(CastKind::FloatToInt, operand, target) => {
            cast!(
                ctx,
                operand,
                target,
                crate::casts::float_to_int,
                ctx.asm_mut()
            )
        }
        Rvalue::Cast(CastKind::IntToFloat, operand, target) => {
            cast!(
                ctx,
                operand,
                target,
                crate::casts::int_to_float,
                ctx.asm_mut()
            )
        }
        Rvalue::NullaryOp(op, ty) => match op {
            NullOp::SizeOf => {
                let ty = ctx.type_from_cache(ctx.monomorphize(*ty));
                conv_usize!(size_of!(ty))
            }
            NullOp::AlignOf => {
                conv_usize!(ldc_u64!(crate::utilis::align_of(
                    ctx.monomorphize(*ty),
                    ctx.tcx()
                )))
            }
            NullOp::OffsetOf(fields) => {
                assert_eq!(fields.len(), 1);
                todo!("Can't calc offset of yet!");
            }
            rustc_middle::mir::NullOp::UbChecks => {
                if ctx.tcx().sess.ub_checks() {
                    CILNode::LdTrue
                } else {
                    CILNode::LdFalse
                }
            }
        },
        Rvalue::Aggregate(aggregate_kind, field_index) => crate::aggregate::handle_aggregate(
            ctx,
            target_location,
            aggregate_kind.as_ref(),
            field_index,
        ),
        Rvalue::Cast(
            CastKind::PointerCoercion(PointerCoercion::ClosureFnPointer(_)),
            ref operand,
            _to_ty,
        ) => match ctx.monomorphize(operand.ty(ctx.body(), ctx.tcx())).kind() {
            TyKind::Closure(def_id, args) => {
                let instance = Instance::resolve_closure(
                    ctx.tcx(),
                    *def_id,
                    args,
                    rustc_middle::ty::ClosureKind::FnOnce,
                )
                .polymorphize(ctx.tcx());
                let call_info = CallInfo::sig_from_instance_(instance, ctx);

                let function_name = crate::utilis::function_name(ctx.tcx().symbol_name(instance));
                let call_site = CallSite::new(None, function_name, call_info.sig().clone(), true);
                CILNode::LDFtn(Box::new(call_site))
            }
            _ => panic!(
                "{} cannot be cast to a fn ptr",
                operand.ty(ctx.body(), ctx.tcx())
            ),
        },
        Rvalue::Cast(CastKind::Transmute, operand, dst) => {
            let dst = ctx.monomorphize(*dst);
            let dst_ty = dst;
            let dst = ctx.type_from_cache(dst);
            let dst_ptr = ctx.asm_mut().nptr(dst);
            let src = operand.ty(&ctx.body().local_decls, ctx.tcx());
            let src = ctx.monomorphize(src);
            let src = ctx.type_from_cache(src);
            match (&src, &dst) {
                (
                    Type::Int(Int::ISize | Int::USize) | Type::Ptr(_) | Type::FnPtr(_),
                    Type::Int(Int::ISize | Int::USize) | Type::Ptr(_) | Type::FnPtr(_),
                ) => handle_operand(operand, ctx).cast_ptr(dst),

                (Type::Int(Int::U16), Type::PlatformChar) => handle_operand(operand, ctx),
                (_, _) => CILNode::TemporaryLocal(Box::new((
                    src,
                    [CILRoot::SetTMPLocal {
                        value: handle_operand(operand, ctx),
                    }]
                    .into(),
                    crate::place::deref_op(
                        crate::place::PlaceTy::Ty(dst_ty),
                        ctx,
                        CILNode::LoadAddresOfTMPLocal.cast_ptr(dst_ptr),
                    ),
                ))),
            }
        }
        Rvalue::ShallowInitBox(operand, dst) => {
            let dst = ctx.monomorphize(*dst);
            let boxed_dst = Ty::new_box(ctx.tcx(), dst);
            //let dst = tycache.type_from_cache(dst, tcx, method_instance);
            let src = operand.ty(&ctx.body().local_decls, ctx.tcx());
            let boxed_dst_type = ctx.type_from_cache(boxed_dst);
            let src = ctx.monomorphize(src);
            assert!(
                !pointer_to_is_fat(dst, ctx.tcx(), ctx.instance()),
                "ERROR: shallow init box used to initialze a fat box!"
            );
            let src = ctx.type_from_cache(src);
            let boxed_ptr = ctx.asm_mut().nptr(boxed_dst_type);
            CILNode::TemporaryLocal(Box::new((
                src,
                [CILRoot::SetTMPLocal {
                    value: handle_operand(operand, ctx),
                }]
                .into(),
                crate::place::deref_op(
                    crate::place::PlaceTy::Ty(boxed_dst),
                    ctx,
                    CILNode::LoadAddresOfTMPLocal.cast_ptr(boxed_ptr),
                ),
            )))
        }
        Rvalue::Cast(CastKind::PointerWithExposedProvenance, operand, target) => {
            //FIXME: the documentation of this cast(https://doc.rust-lang.org/nightly/std/ptr/fn.from_exposed_addr.html) is a bit confusing,
            //since this seems to be something deeply linked to the rust memory model.
            // I assume this to be ALWAYS equivalent to `usize as *const/mut T`, but this may not always be the case.
            // If something breaks in the fututre, this is a place that needs checking.
            let target = ctx.monomorphize(*target);
            let target = ctx.type_from_cache(target);
            // Cast from usize/isize to any *T is a NOP, so we just have to load the operand.
            handle_operand(operand, ctx).cast_ptr(target)
        }
        Rvalue::Cast(CastKind::PointerExposeProvenance, operand, target) => {
            //FIXME: the documentation of this cast(https://doc.rust-lang.org/nightly/std/primitive.pointer.html#method.expose_addrl) is a bit confusing,
            //since this seems to be something deeply linked to the rust memory model.
            // I assume this to be ALWAYS equivalent to `*const/mut T as usize`, but this may not always be the case.
            // If something breaks in the fututre, this is a place that needs checking.
            let target = ctx.monomorphize(*target);
            let target = ctx.type_from_cache(target);
            // Cast to usize/isize from any *T is a NOP, so we just have to load the operand.

            let val = handle_operand(operand, ctx);
            match target {
                Type::Int(Int::USize | Int::ISize) | Type::Ptr(_) | Type::FnPtr(_) => {
                    val.cast_ptr(target)
                }
                Type::Int(Int::U64 | Int::I64) => crate::casts::int_to_int(
                    Type::Int(Int::USize),
                    &target,
                    val.cast_ptr(Type::Int(Int::USize)),
                    ctx.asm_mut(),
                ),
                _ => todo!("Can't cast using `PointerExposeProvenance` to {target:?}"),
            }
        }
        Rvalue::Cast(CastKind::FloatToFloat, operand, target) => {
            let target = ctx.monomorphize(*target);
            let target = ctx.type_from_cache(target);
            let mut ops = handle_operand(operand, ctx);
            match target {
                Type::Float(Float::F32) => ops = CILNode::ConvF32(ops.into()),
                Type::Float(Float::F64) => ops = CILNode::ConvF64(ops.into()),
                _ => panic!("Can't preform a FloatToFloat cast to type {target:?}"),
            }
            ops
        }
        Rvalue::Cast(
            CastKind::PointerCoercion(PointerCoercion::ReifyFnPointer),
            operand,
            _target,
        ) => {
            let operand_ty = operand.ty(ctx.body(), ctx.tcx());
            operand
                .constant()
                .expect("function must be constant in order to take its adress!");
            let operand_ty = ctx.monomorphize(operand_ty);

            let (instance, _subst_ref) = if let TyKind::FnDef(def_id, subst_ref) = operand_ty.kind()
            {
                let subst = ctx.monomorphize(*subst_ref);
                let env = ParamEnv::reveal_all();
                let Some(instance) = Instance::try_resolve(ctx.tcx(), env, *def_id, subst)
                    .expect("Invalid function def")
                else {
                    panic!("ERROR: Could not get function instance. fn type:{operand_ty:?}")
                };
                (instance, subst_ref)
            } else {
                todo!("Trying to call a type which is not a function definition!");
            };
            let function_name = crate::utilis::function_name(ctx.tcx().symbol_name(instance));
            let function_sig = crate::function_sig::sig_from_instance_(instance, ctx)
                .expect("Could not get function signature when trying to get a function pointer!");
            //FIXME: propely handle `#[track_caller]`
            let call_site = CallSite::new(None, function_name, function_sig, true);
            CILNode::LDFtn(call_site.into())
        }

        Rvalue::Discriminant(place) => {
            let addr = crate::place::place_adress(place, ctx);
            let owner_ty = ctx.monomorphize(place.ty(ctx.body(), ctx.tcx()).ty);
            let owner = ctx.type_from_cache(owner_ty);

            let layout = ctx.layout_of(owner_ty);
            let target = ctx.type_from_cache(owner_ty.discriminant_ty(ctx.tcx()));
            let (disrc_type, _) = crate::utilis::adt::enum_tag_info(layout.layout, ctx.asm_mut());
            let owner = if let Type::ClassRef(dotnet_type) = owner {
                dotnet_type
            } else {
                eprintln!("Can't get the discirminant of type {owner_ty:?}, because it is a zst. Size:{} Discr type:{:?}",layout.layout.size.bytes(), owner_ty.discriminant_ty(ctx.tcx()));
                return crate::casts::int_to_int(
                    Type::Int(Int::I32),
                    &target,
                    ldc_i32!(0),
                    ctx.asm_mut(),
                );
            };

            if disrc_type == Type::Void {
                // Just alwways return 0 if the discriminat type is `()` - this seems to work, and be what rustc expects. Wierd, but OK.
                crate::casts::int_to_int(Type::Int(Int::I32), &target, ldc_i32!(0), ctx.asm_mut())
            } else {
                crate::casts::int_to_int(
                    disrc_type,
                    &target,
                    crate::utilis::adt::get_discr(layout.layout, addr, owner, owner_ty, ctx),
                    ctx.asm_mut(),
                )
            }
        }
        Rvalue::Len(operand) => {
            let ty = ctx.monomorphize(operand.ty(ctx.body(), ctx.tcx()));
            match ty.ty.kind() {
                TyKind::Slice(inner) => {
                    let slice_tpe = fat_ptr_to(*inner, ctx);
                    let descriptor = FieldDescriptor::new(
                        slice_tpe,
                        cilly::v2::Type::Int(Int::USize),
                        crate::METADATA.into(),
                    );
                    let addr = crate::place::place_address_raw(operand, ctx);
                    assert!(
                        !matches!(addr, CILNode::LDLoc(_)),
                        "improper addr {addr:?}. operand:{operand:?}"
                    );
                    ld_field!(addr, descriptor)
                }
                TyKind::Array(_ty, length) => {
                    conv_usize!(ldc_u64!(crate::utilis::try_resolve_const_size(
                        ctx.monomorphize(*length)
                    )
                    .unwrap() as u64))
                }
                _ => todo!("Get length of type {ty:?}"),
            }
        }
        Rvalue::Repeat(operand, times) => repeat(rvalue, ctx, operand, *times),
        Rvalue::ThreadLocalRef(def_id) => {
            if !def_id.is_local() && ctx.tcx().needs_thread_local_shim(*def_id) {
                let _instance = Instance {
                    def: InstanceKind::ThreadLocalShim(*def_id),
                    args: GenericArgs::empty(),
                };
                // Call instance
                todo!("Thread locals with shims unsupported!")
            } else {
                let alloc_id = ctx.tcx().reserve_and_set_static_alloc(*def_id);
                let rvalue_ty = rvalue.ty(ctx.body(), ctx.tcx());
                let rvalue_type = ctx.type_from_cache(rvalue_ty);
                CILNode::LoadGlobalAllocPtr {
                    alloc_id: alloc_id.0.into(),
                }
                .cast_ptr(rvalue_type)
            }
        }
        Rvalue::Cast(rustc_middle::mir::CastKind::FnPtrToPtr, operand, target) => {
            let target = ctx.type_from_cache(*target);
            handle_operand(operand, ctx).cast_ptr(target)
        }
        Rvalue::Cast(rustc_middle::mir::CastKind::DynStar, _, _) => {
            todo!("Unusported cast kind:DynStar")
        }
    }
}
fn repeat<'tcx>(
    rvalue: &Rvalue<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    element: &Operand<'tcx>,
    times: rustc_middle::ty::Const<'tcx>,
) -> CILNode {
    // Get the type of the operand
    let element_ty = ctx.monomorphize(element.ty(ctx.body(), ctx.tcx()));
    let element_type = ctx.type_from_cache(element_ty);
    let element = handle_operand(element, ctx);
    // Array size
    let times = ctx.monomorphize(times);
    let times = times
        .try_eval_target_usize(ctx.tcx(), ParamEnv::reveal_all())
        .expect("Could not evalute array size as usize.");
    // Array type
    let array = ctx.monomorphize(rvalue.ty(ctx.body(), ctx.tcx()));
    let array = ctx.type_from_cache(array);
    let array_dotnet = array.clone().as_class_ref().expect("Invalid array type.");
    // Check if the element is byte sized. If so, use initblk to quickly initialize this array.
    if crate::utilis::compiletime_sizeof(element_ty, ctx.tcx()) == 1 {
        let val = Box::new(CILNode::TemporaryLocal(Box::new((
            element_type,
            vec![CILRoot::SetTMPLocal { value: element }].into(),
            CILNode::LDIndU8 {
                ptr: Box::new(
                    CILNode::LoadAddresOfTMPLocal.cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::U8))),
                ),
            },
        ))));
        let init = CILRoot::InitBlk {
            dst: Box::new(
                CILNode::LoadAddresOfTMPLocal.cast_ptr(ctx.asm_mut().nptr(Type::Int(Int::U8))),
            ),
            val,
            count: Box::new(conv_usize!(ldc_u64!(times))),
        };
        return CILNode::TemporaryLocal(Box::new((
            array,
            vec![init].into(),
            CILNode::LoadTMPLocal,
        )));
    }
    // Check if there are more than 16 elements. If so, use mecmpy to accelerate initialzation
    if times > 16 {
        let mut branches = Vec::new();
        for idx in 0..16 {
            branches.push(CILRoot::Call {
                site: Box::new(CallSite::new(
                    Some(array_dotnet),
                    "set_Item".into(),
                    FnSig::new(
                        [
                            ctx.asm_mut().nref(array),
                            Type::Int(Int::USize),
                            element_type,
                        ],
                        Type::Void,
                    ),
                    false,
                )),
                args: [
                    CILNode::LoadAddresOfTMPLocal,
                    conv_usize!(ldc_u64!(idx)),
                    element.clone(),
                ]
                .into(),
            });
        }
        let mut curr_len = 16;

        while curr_len < times {
            // Copy curr_len elements if possible, otherwise this is the last iteration, so copy the reminder.
            let curr_copy_size = curr_len.min(times - curr_len);
            // Copy curr_copy_size elements from the start of the array, starting at curr_len(the ammount of already initialized buffers)
            branches.push(CILRoot::CpBlk {
                dst: Box::new(
                    CILNode::MRefToRawPtr(Box::new(CILNode::LoadAddresOfTMPLocal))
                        + conv_usize!(ldc_u64!(curr_len)),
                ),
                src: Box::new(CILNode::LoadAddresOfTMPLocal),
                len: Box::new(
                    conv_usize!(ldc_u64!(curr_copy_size))
                        * conv_usize!(size_of!(element_type)),
                ),
            });
            curr_len *= 2;
        }
        let branches: Box<_> = branches.into();
        CILNode::TemporaryLocal(Box::new((array, branches, CILNode::LoadTMPLocal)))
    } else {
        let mut branches = Vec::new();
        for idx in 0..times {
            branches.push(CILRoot::Call {
                site: Box::new(CallSite::new(
                    Some(array_dotnet),
                    "set_Item".into(),
                    FnSig::new(
                        [
                            ctx.asm_mut().nref(array),
                            Type::Int(Int::USize),
                            element_type,
                        ],
                        Type::Void,
                    ),
                    false,
                )),
                args: [
                    CILNode::LoadAddresOfTMPLocal,
                    conv_usize!(ldc_u64!(idx)),
                    element.clone(),
                ]
                .into(),
            });
        }
        let branches: Box<_> = branches.into();
        CILNode::TemporaryLocal(Box::new((array, branches, CILNode::LoadTMPLocal)))
    }
}
fn ptr_to_ptr<'tcx>(
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    operand: &Operand<'tcx>,
    dst: Ty<'tcx>,
) -> CILNode {
    let target = ctx.monomorphize(dst);
    let target_pointed_to = match target.kind() {
        TyKind::RawPtr(typ, _) => typ,
        TyKind::Ref(_, inner, _) => inner,
        _ => panic!("Type is not ptr {target:?}."),
    };
    let source = ctx.monomorphize(operand.ty(ctx.body(), ctx.tcx()));
    let source_pointed_to = match source.kind() {
        TyKind::RawPtr(typ, _) => *typ,
        TyKind::Ref(_, inner, _) => *inner,
        _ => panic!("Type is not ptr {target:?}."),
    };
    let source_type = ctx.type_from_cache(source);
    let target_type = ctx.type_from_cache(target);

    let src_fat = pointer_to_is_fat(source_pointed_to, ctx.tcx(), ctx.instance());
    let target_fat = pointer_to_is_fat(*target_pointed_to, ctx.tcx(), ctx.instance());
    match (src_fat, target_fat) {
        (true, true) => {
            let parrent = handle_operand(operand, ctx);

            let target_ptr = ctx.asm_mut().nptr(target_type);
            crate::place::deref_op(
                crate::place::PlaceTy::Ty(target),
                ctx,
                CILNode::TemporaryLocal(Box::new((
                    source_type,
                    [CILRoot::SetTMPLocal { value: parrent }].into(),
                    Box::new(CILNode::LoadAddresOfTMPLocal).cast_ptr(target_ptr),
                ))),
            )
        }
        (true, false) => CILNode::TemporaryLocal(Box::new((
            source_type,
            [CILRoot::SetTMPLocal {
                value: handle_operand(operand, ctx),
            }]
            .into(),
            ld_field!(
                CILNode::LoadAddresOfTMPLocal,
                FieldDescriptor::new(
                    get_type(source, ctx).as_class_ref().unwrap(),
                    ctx.asm_mut().nptr(cilly::v2::Type::Void),
                    crate::DATA_PTR.into(),
                )
            )
            .cast_ptr(target_type),
        ))),
        (false, true) => {
            panic!("ERROR: a non-unsizing cast turned a sized ptr into an unsized one")
        }
        _ => handle_operand(operand, ctx).cast_ptr(target_type),
    }
}
