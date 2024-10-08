use crate::{assembly::MethodCompileCtx, operand::handle_operand, place::place_set};
use cilly::{
    and, call,
    call_site::CallSite,
    cil_node::CILNode,
    cil_root::CILRoot,
    conv_i16, conv_i32, conv_i8, conv_isize, conv_u16, conv_u32, conv_u64, conv_u8, conv_usize,
    ldc_i32, ldc_u32, ldc_u64, or, rem_un, size_of, sub, shr_un, shl,
    v2::{ClassRef, FnSig, Int},
    Type,
};
use cilly::cil_node::CallOpArgs;
use rustc_middle::{
    mir::{Operand, Place},
    ty::Instance,
};
use rustc_span::source_map::Spanned;
pub fn ctpop<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,

    call_instance: Instance<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
) -> CILRoot {
    debug_assert_eq!(
        args.len(),
        1,
        "The intrinsic `ctpop` MUST take in exactly 1 argument!"
    );
    let tpe = ctx.type_from_cache(
        ctx.monomorphize(
            call_instance.args[0]
                .as_type()
                .expect("needs_drop works only on types!"),
        ),
    );
    let bit_operations = ClassRef::bit_operations(ctx.asm_mut());
    let bit_operations = Some(bit_operations);
    let operand = handle_operand(&args[0].node, ctx);
    place_set(
        destination,
        match tpe {
            Type::Int(Int::U64) => conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "PopCount".into(),
                    FnSig::new([Type::Int(Int::U64)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [operand]
            )),
            Type::Int(Int::I64) => conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "PopCount".into(),
                    FnSig::new([Type::Int(Int::U64)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_u64!(operand)]
            )),
            Type::Int(Int::U32) => conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "PopCount".into(),
                    FnSig::new([Type::Int(Int::U32)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [operand]
            )),

            Type::Int(Int::U8 | Int::U16 | Int::I8 | Int::I16 | Int::I32) => {
                conv_u32!(call!(
                    CallSite::boxed(
                        bit_operations,
                        "PopCount".into(),
                        FnSig::new([Type::Int(Int::U32)].into(), Type::Int(Int::I32)),
                        true,
                    ),
                    [conv_u32!(operand)]
                ))
            }
            Type::Int(Int::USize) => conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "PopCount".into(),
                    FnSig::new([Type::Int(Int::USize)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [operand]
            )),
            Type::Int(Int::ISize) => conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "PopCount".into(),
                    FnSig::new([Type::Int(Int::USize)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_isize!(operand)]
            )),
            Type::Int(Int::U128) => crate::casts::int_to_int(
                Type::Int(Int::U128),
                Type::Int(Int::U32),
                call!(
                    CallSite::new_extern(
                        ClassRef::uint_128(ctx.asm_mut()),
                        "PopCount".into(),
                        FnSig::new([Type::Int(Int::U128)].into(), Type::Int(Int::U128)),
                        true,
                    ),
                    [operand]
                ),
                ctx.asm_mut(),
            ),
            Type::Int(Int::I128) => crate::casts::int_to_int(
                Type::Int(Int::I128),
                Type::Int(Int::U32),
                call!(
                    CallSite::new_extern(
                        ClassRef::int_128(ctx.asm_mut()),
                        "PopCount".into(),
                        FnSig::new([Type::Int(Int::I128)].into(), Type::Int(Int::I128)),
                        true,
                    ),
                    [operand]
                ),
                ctx.asm_mut(),
            ),
            _ => todo!("Unsported pop count type {tpe:?}"),
        },
        ctx,
    )
}
pub fn ctlz<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    call_instance: Instance<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
) -> CILRoot {
    debug_assert_eq!(
        args.len(),
        1,
        "The intrinsic `ctlz` MUST take in exactly 1 argument!"
    );
    let bit_operations = ClassRef::bit_operations(ctx.asm_mut());
    let bit_operations = Some(bit_operations);

    let tpe = ctx.monomorphize(
        call_instance.args[0]
            .as_type()
            .expect("needs_drop works only on types!"),
    );
    let tpe = ctx.type_from_cache(tpe);
    // TODO: this assumes a 64 bit system!
    let sub = match tpe {
        Type::Int(Int::ISize | Int::USize) | Type::Ptr(_) => {
            ldc_i32!(64) - (size_of!(tpe) * ldc_i32!(8))
        }
        Type::Int(Int::I64 | Int::U64) => ldc_i32!(0),
        Type::Int(Int::I32 | Int::U32) => ldc_i32!(32),
        Type::Int(Int::I16 | Int::U16) => ldc_i32!(48),
        Type::Int(Int::I8 | Int::U8) => ldc_i32!(56),
        Type::Int(Int::I128) => {
            return place_set(
                destination,
                conv_u32!(call!(
                    CallSite::new_extern(
                        ClassRef::int_128(ctx.asm_mut()),
                        "LeadingZeroCount".into(),
                        FnSig::new([Type::Int(Int::I128)].into(), Type::Int(Int::I128)),
                        true
                    ),
                    [handle_operand(&args[0].node, ctx)]
                )),
                ctx,
            )
        }
        Type::Int(Int::U128) => {
            return place_set(
                destination,
                conv_u32!(call!(
                    CallSite::new_extern(
                        ClassRef::uint_128(ctx.asm_mut()),
                        "LeadingZeroCount".into(),
                        FnSig::new([Type::Int(Int::U128)].into(), Type::Int(Int::U128)),
                        true
                    ),
                    [handle_operand(&args[0].node, ctx)]
                )),
                ctx,
            )
        }
        _ => todo!("Can't `ctlz`  type {tpe:?} yet!"),
    };
    place_set(
        destination,
        conv_u32!(sub!(
            call!(
                CallSite::boxed(
                    bit_operations,
                    "LeadingZeroCount".into(),
                    FnSig::new([Type::Int(Int::U64)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_u64!(handle_operand(&args[0].node, ctx))]
            ),
            sub
        )),
        ctx,
    )
}
pub fn cttz<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    call_instance: Instance<'tcx>,
) -> CILRoot {
    debug_assert_eq!(
        args.len(),
        1,
        "The intrinsic `ctlz` MUST take in exactly 1 argument!"
    );
    let bit_operations = ClassRef::bit_operations(ctx.asm_mut());
    let tpe = ctx.monomorphize(
        call_instance.args[0]
            .as_type()
            .expect("needs_drop works only on types!"),
    );
    let tpe = ctx.type_from_cache(tpe);
    let bit_operations = Some(bit_operations);
    let operand = handle_operand(&args[0].node, ctx);
    match tpe {
        Type::Int(Int::I8) => {
            let value_calc = conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "TrailingZeroCount".into(),
                    FnSig::new([Type::Int(Int::I32)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_i32!(operand)]
            ));
            place_set(
                destination,
                call!(
                    CallSite::new_extern(
                        ClassRef::math(ctx.asm_mut()),
                        "Min".into(),
                        FnSig::new(
                            [Type::Int(Int::U32), Type::Int(Int::U32)].into(),
                            Type::Int(Int::U32)
                        ),
                        true
                    ),
                    [value_calc, ldc_u32!(i8::BITS)]
                ),
                ctx,
            )
        }
        Type::Int(Int::I16) => {
            let value_calc = conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "TrailingZeroCount".into(),
                    FnSig::new([Type::Int(Int::I32)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_i32!(operand)]
            ));
            place_set(
                destination,
                call!(
                    CallSite::new_extern(
                        ClassRef::math(ctx.asm_mut()),
                        "Min".into(),
                        FnSig::new(
                            [Type::Int(Int::U32), Type::Int(Int::U32)].into(),
                            Type::Int(Int::U32)
                        ),
                        true
                    ),
                    [value_calc, ldc_u32!(i16::BITS)]
                ),
                ctx,
            )
        }
        Type::Int(Int::U8) => {
            let value_calc = conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "TrailingZeroCount".into(),
                    FnSig::new([Type::Int(Int::U32)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_u32!(operand)]
            ));
            place_set(
                destination,
                call!(
                    CallSite::new_extern(
                        ClassRef::math(ctx.asm_mut()),
                        "Min".into(),
                        FnSig::new(
                            [Type::Int(Int::U32), Type::Int(Int::U32)].into(),
                            Type::Int(Int::U32)
                        ),
                        true
                    ),
                    [value_calc, ldc_u32!(u8::BITS)]
                ),
                ctx,
            )
        }
        Type::Int(Int::U16) => {
            let value_calc = conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "TrailingZeroCount".into(),
                    FnSig::new([Type::Int(Int::U32)].into(), Type::Int(Int::I32)),
                    true,
                ),
                [conv_u32!(operand)]
            ));
            place_set(
                destination,
                call!(
                    CallSite::new_extern(
                        ClassRef::math(ctx.asm_mut()),
                        "Min".into(),
                        FnSig::new(
                            [Type::Int(Int::U32), Type::Int(Int::U32)].into(),
                            Type::Int(Int::U32)
                        ),
                        true
                    ),
                    [value_calc, ldc_u32!(u16::BITS)]
                ),
                ctx,
            )
        }
        Type::Int(Int::I128) => place_set(
            destination,
            conv_u32!(call!(
                CallSite::new_extern(
                    ClassRef::int_128(ctx.asm_mut()),
                    "TrailingZeroCount".into(),
                    FnSig::new([Type::Int(Int::I128)].into(), Type::Int(Int::I128)),
                    true
                ),
                [handle_operand(&args[0].node, ctx)]
            )),
            ctx,
        ),
        Type::Int(Int::U128) => place_set(
            destination,
            conv_u32!(call!(
                CallSite::new_extern(
                    ClassRef::uint_128(ctx.asm_mut()),
                    "TrailingZeroCount".into(),
                    FnSig::new([Type::Int(Int::U128)].into(), Type::Int(Int::U128)),
                    true
                ),
                [handle_operand(&args[0].node, ctx)]
            )),
            ctx,
        ),
        _ => place_set(
            destination,
            conv_u32!(call!(
                CallSite::boxed(
                    bit_operations,
                    "TrailingZeroCount".into(),
                    FnSig::new([tpe].into(), Type::Int(Int::I32)),
                    true,
                ),
                [operand]
            )),
            ctx,
        ),
    }
}
pub fn rotate_left<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    call_instance: Instance<'tcx>,
) -> CILRoot {
    debug_assert_eq!(
        args.len(),
        2,
        "The intrinsic `rotate_left` MUST take in exactly 2 arguments!"
    );
    let val_tpe = ctx.monomorphize(
        call_instance.args[0]
            .as_type()
            .expect("needs_drop works only on types!"),
    );
    let val_tpe = ctx.type_from_cache(val_tpe);
    let val = handle_operand(&args[0].node, ctx);
    let rot = handle_operand(&args[1].node, ctx);
    match val_tpe {
        Type::Int(Int::U8) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::byte(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U8), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U8)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U16) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::uint16(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U16), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U16)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U32) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U32), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U64) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U64), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U64)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::USize) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::USize), Type::Int(Int::I32)].into(),
                        Type::Int(Int::USize)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I8) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::sbyte(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::I8), Type::Int(Int::I32)].into(),
                        Type::Int(Int::I8)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I16) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::int16(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::I16), Type::Int(Int::I32)].into(),
                        Type::Int(Int::I16)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I32) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U32), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [conv_u32!(val), conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I64) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U64), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [conv_u64!(val), conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::ISize) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::USize), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [conv_usize!(val), conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U128) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::uint_128(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::U128), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U128)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I128) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::int_128(ctx.asm_mut()),
                    "RotateLeft".into(),
                    FnSig::new(
                        [Type::Int(Int::I128), Type::Int(Int::I32)].into(),
                        Type::Int(Int::I128)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        _ => todo!("Can't ror {val_tpe:?}"),
    }
}
pub fn rotate_right<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    call_instance: Instance<'tcx>,
) -> CILRoot {
    debug_assert_eq!(
        args.len(),
        2,
        "The  `rotate_right` MUST take in exactly 2 arguments!"
    );
    let val_tpe = ctx.monomorphize(
        call_instance.args[0]
            .as_type()
            .expect("needs_drop works only on types!"),
    );
    let val_tpe = ctx.type_from_cache(val_tpe);
    let val = handle_operand(&args[0].node, ctx);
    let rot = handle_operand(&args[1].node, ctx);
    match val_tpe {
        Type::Int(Int::U16) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::uint16(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U16), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U16)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U8) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::byte(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U8), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U8)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U32) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U32), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U64) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U64), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U64)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::USize) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::USize), Type::Int(Int::I32)].into(),
                        Type::Int(Int::USize)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I8) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::sbyte(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::I8), Type::Int(Int::I32)].into(),
                        Type::Int(Int::I8)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I16) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::int16(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::I16), Type::Int(Int::I32)].into(),
                        Type::Int(Int::I16)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I32) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U32), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [conv_u32!(val), conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I64) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U64), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [conv_u64!(val), conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::ISize) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::bit_operations(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::USize), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U32)
                    ),
                    true
                ),
                [conv_usize!(val), conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::U128) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::uint_128(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::U128), Type::Int(Int::I32)].into(),
                        Type::Int(Int::U128)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        Type::Int(Int::I128) => place_set(
            destination,
            call!(
                CallSite::new_extern(
                    ClassRef::int_128(ctx.asm_mut()),
                    "RotateRight".into(),
                    FnSig::new(
                        [Type::Int(Int::I128), Type::Int(Int::I32)].into(),
                        Type::Int(Int::I128)
                    ),
                    true
                ),
                [val, conv_i32!(rot)]
            ),
            ctx,
        ),
        _ => todo!("Can't ror {val_tpe:?}"),
    }
}
pub fn bitreverse_u8(byte: CILNode) -> CILNode {
    conv_u8!(rem_un!(
        (and!(
            conv_u64!(byte) * ldc_u64!(0x0002_0202_0202),
            ldc_u64!(0x0108_8442_2010)
        )),
        ldc_u64!(1023)
    ))
}
fn bitreverse_u16(ushort: CILNode) -> CILNode {
    let lower = bitreverse_u8(conv_u8!(ushort.clone()));
    let upper = bitreverse_u8(conv_u8!(shr_un!(ushort, ldc_u32!(8))));
    or!(shl!(conv_u16!(lower), ldc_u32!(8)), conv_u16!(upper))
}
fn bitreverse_u32(uint: CILNode) -> CILNode {
    let lower = bitreverse_u16(conv_u16!(uint.clone()));
    let upper = bitreverse_u16(conv_u16!(shr_un!(uint, ldc_u32!(16))));
    or!(shl!(conv_u32!(lower), ldc_u32!(16)), conv_u32!(upper))
}
fn bitreverse_u64(ulong: CILNode) -> CILNode {
    let lower = bitreverse_u32(conv_u32!(ulong.clone()));
    let upper = bitreverse_u32(conv_u32!(shr_un!(ulong, ldc_u32!(32))));
    or!(shl!(conv_u64!(lower), ldc_u32!(32)), conv_u64!(upper))
}

fn bitreverse_u128<'tcx>(ullong: CILNode, ctx: &mut MethodCompileCtx<'tcx, '_>) -> CILNode {
    let lower = crate::casts::int_to_int(
        Type::Int(Int::U128),
        Type::Int(Int::U64),
        ullong.clone(),
        ctx.asm_mut(),
    );
    let shifted = call!(
        CallSite::new_extern(
            ClassRef::uint_128(ctx.asm_mut()),
            "op_UnsignedRightShift".into(),
            FnSig::new(
                [Type::Int(Int::U128), Type::Int(Int::I32)].into(),
                Type::Int(Int::U128)
            ),
            true
        ),
        [ullong, ldc_i32!(64)]
    );
    let upper = crate::casts::int_to_int(
        Type::Int(Int::U128),
        Type::Int(Int::U64),
        shifted,
        ctx.asm_mut(),
    );

    CILNode::NewObj(Box::new(CallOpArgs {
        site: CallSite::boxed(
            Some(ClassRef::uint_128(ctx.asm_mut())),
            ".ctor".into(),
            FnSig::new(
                [Type::Int(Int::U128), Type::Int(Int::U64), Type::Int(Int::U64)].into(),
                Type::Void
            ),
            false,
        ),
        args: [bitreverse_u64(lower), bitreverse_u64(upper)].into(),
    }))
}

pub fn bitreverse<'tcx>(
    args: &[Spanned<Operand<'tcx>>],
    destination: &Place<'tcx>,
    ctx: &mut MethodCompileCtx<'tcx, '_>,
    call_instance: Instance<'tcx>,
) -> CILRoot {
    debug_assert_eq!(
        args.len(),
        1,
        "The  `bitreverse` MUST take in exactly 1 argument!"
    );
    let val_tpe = ctx.monomorphize(
        call_instance.args[0]
            .as_type()
            .expect("needs_drop works only on types!"),
    );
    let val_tpe = ctx.type_from_cache(val_tpe);
    let val = handle_operand(&args[0].node, ctx);
    place_set(
        destination,
        match val_tpe {
            Type::Int(Int::U8) => bitreverse_u8(val),
            Type::Int(Int::I8) => conv_i8!(bitreverse_u8(val)),
            Type::Int(Int::U16) => bitreverse_u16(val),
            Type::Int(Int::I16) => conv_i16!(bitreverse_u16(conv_u16!(val))),
            Type::Int(Int::U32) => bitreverse_u32(val),
            Type::Int(Int::I32) => crate::casts::int_to_int(
                Type::Int(Int::U32),
                Type::Int(Int::I32),
                bitreverse_u32(crate::casts::int_to_int(
                    Type::Int(Int::I32),
                    Type::Int(Int::U32),
                    val,
                    ctx.asm_mut(),
                )),
                ctx.asm_mut(),
            ),
            Type::Int(Int::U64) => bitreverse_u64(val),
            Type::Int(Int::I64) => crate::casts::int_to_int(
                Type::Int(Int::U64),
                Type::Int(Int::I64),
                bitreverse_u64(crate::casts::int_to_int(
                    Type::Int(Int::I64),
                    Type::Int(Int::U64),
                    val,
                    ctx.asm_mut(),
                )),
                ctx.asm_mut(),
            ),
            Type::Int(Int::U128) => bitreverse_u128(val, ctx),
            Type::Int(Int::I128) => crate::casts::int_to_int(
                Type::Int(Int::U128),
                Type::Int(Int::I128),
                bitreverse_u128(crate::casts::int_to_int(
                    Type::Int(Int::I128),
                    Type::Int(Int::U128),
                    val,
                    ctx.asm_mut(),
                ), ctx),
                ctx.asm_mut(),
            ),

            _ => todo!("can't yet bitreverse {val_tpe:?}"),
        },
        ctx,
    )
}
