use crate::v2::{Assembly, ClassRef, FnSig, Type};
use crate::{
    call,
    call_site::CallSite,
    cil_node::{CILNode, CallOpArgs},
    field_desc::FieldDescriptor,
    static_field_desc::StaticFieldDescriptor,
    AsmString, IString,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CILRoot {
    STLoc {
        local: u32,
        tree: CILNode,
    },
    BTrue {
        target: u32,
        sub_target: u32,
        cond: CILNode,
    },
    BFalse {
        target: u32,
        sub_target: u32,
        cond: CILNode,
    },
    BEq {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BLt {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BLtUn {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BGt {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BGtUn {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BLe {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BGe {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    BNe {
        target: u32,
        sub_target: u32,
        a: Box<CILNode>,
        b: Box<CILNode>,
    },
    GoTo {
        target: u32,
        sub_target: u32,
    },

    Call {
        site: Box<CallSite>,
        args: Box<[CILNode]>,
    },
    SetField {
        addr: Box<CILNode>,
        value: Box<CILNode>,
        desc: Box<FieldDescriptor>,
    },
    SetTMPLocal {
        value: CILNode,
    },
    CpBlk {
        dst: Box<CILNode>,
        src: Box<CILNode>,
        len: Box<CILNode>,
    },
    STIndI8(CILNode, CILNode),
    STIndI16(CILNode, CILNode),
    STIndI32(CILNode, CILNode),
    STIndI64(CILNode, CILNode),
    STIndISize(CILNode, CILNode),
    // addr, val, points_to
    STIndPtr(CILNode, CILNode, Box<Type>),
    STIndF64(CILNode, CILNode),
    STIndF32(CILNode, CILNode),
    STObj {
        tpe: Box<Type>,
        addr_calc: Box<CILNode>,
        value_calc: Box<CILNode>,
    },
    STArg {
        arg: u32,
        tree: CILNode,
    },
    Break,
    Nop,
    InitBlk {
        dst: Box<CILNode>,
        val: Box<CILNode>,
        count: Box<CILNode>,
    },
    CallVirt {
        site: Box<CallSite>,
        args: Box<[CILNode]>,
    },
    Ret {
        tree: CILNode,
    },
    Pop {
        tree: CILNode,
    },
    VoidRet,
    Throw(CILNode),
    ReThrow,
    CallI {
        sig: Box<FnSig>,
        fn_ptr: Box<CILNode>,
        args: Box<[CILNode]>,
    },
    JumpingPad {
        source: u32,
        target: u32,
    },
    SetStaticField {
        descr: Box<StaticFieldDescriptor>,
        value: CILNode,
    },
    SourceFileInfo(SFI),
    OptimizedSourceFileInfo(std::ops::Range<u64>, std::ops::Range<u64>, AsmString),
    /// Marks the inner pointer operation as volatile.
    Volatile(Box<Self>),
}
pub type SFI = Box<(std::ops::Range<u64>, std::ops::Range<u64>, IString)>;
impl CILRoot {
    #[must_use]
    pub fn throw(msg: &str, asm: &mut Assembly) -> Self {
        let class = ClassRef::exception(asm);

        let name = ".ctor".to_owned().into();
        let signature = FnSig::new(Box::new([class.into(), Type::PlatformString]), Type::Void);
        Self::Throw(CILNode::NewObj(Box::new(CallOpArgs {
            site: CallSite::boxed(Some(class), name, signature, false),
            args: [CILNode::LdStr(msg.into())].into(),
        })))
    }
    #[must_use]
    pub fn debug(msg: &str, asm: &mut Assembly) -> Self {
        let class = ClassRef::console(asm);

        let name = "WriteLine".to_owned().into();
        let signature = FnSig::new(Box::new([Type::PlatformString]), Type::Void);
        let message = tiny_message(msg, asm);
        Self::Call {
            site: Box::new(CallSite::new_extern(class, name, signature, true)),
            args: [message].into(),
        }
    }
    pub fn targets(&self, targets: &mut Vec<(u32, u32)>) {
        match self {
            Self::BTrue {
                target, sub_target, ..
            }
            | Self::BFalse {
                target, sub_target, ..
            }
            | Self::BEq {
                target, sub_target, ..
            }
            | Self::BNe {
                target, sub_target, ..
            }
            | Self::GoTo { target, sub_target } => {
                targets.push((*target, *sub_target));
            }
            _ => (),
        }
    }
    pub fn fix_for_exception_handler(&mut self, id: u32) {
        match self {
            Self::BTrue {
                target, sub_target, ..
            }
            | Self::BFalse {
                target, sub_target, ..
            }
            | Self::BEq {
                target, sub_target, ..
            }
            | Self::BNe {
                target, sub_target, ..
            }
            | Self::BLt {
                target, sub_target, ..
            }
            | Self::BLtUn {
                target, sub_target, ..
            }
            | Self::BLe {
                target, sub_target, ..
            }
            | Self::BGt {
                target, sub_target, ..
            }
            | Self::BGtUn {
                target, sub_target, ..
            }
            | Self::BGe {
                target, sub_target, ..
            }
            | Self::GoTo { target, sub_target } => {
                assert_eq!(
                    *sub_target, 0,
                    "An exception handler can't contain inner exception handler!"
                );
                *sub_target = *target;
                *target = id;
            }
            _ => (),
        }
    }
    #[must_use]
    pub fn sheed_trees(mut self) -> Vec<Self> {
        let iter_mut = (&mut self).into_iter();
        let mut res: Vec<CILRoot> = iter_mut
            .flat_map(|tree| match tree {
                crate::cil_iter_mut::CILIterElemMut::Node(node) => match node {
                    CILNode::SubTrees(tm) => {
                        let (trees, main) = tm.as_mut();
                        let vec = trees.to_vec();
                        let iter = vec.into_iter();
                        let trees = iter.flat_map(|tree| tree.sheed_trees()).collect();
                        *node = *main.clone();
                        trees
                    }
                    _ => vec![],
                },
                _ => vec![],
            })
            .collect();
        res.push(self);
        res
    }
    pub fn allocate_tmps(
        &mut self,
        curr_loc: Option<u32>,
        locals: &mut Vec<(Option<IString>, Type)>,
    ) {
        match self {
            Self::Volatile(inner) => inner.allocate_tmps(curr_loc, locals),
            Self::SourceFileInfo(_) => (),
            Self::OptimizedSourceFileInfo(_, _, _) => (),
            Self::STLoc { tree, .. } => {
                tree.allocate_tmps(curr_loc, locals);
            }
            Self::BTrue { cond: ops, .. } => ops.allocate_tmps(curr_loc, locals),
            Self::BFalse { cond: ops, .. } => ops.allocate_tmps(curr_loc, locals),
            Self::BEq { a, b, .. }
            | Self::BNe { a, b, .. }
            | Self::BLt { a, b, .. }
            | Self::BLtUn { a, b, .. }
            | Self::BGt { a, b, .. }
            | Self::BGtUn { a, b, .. }
            | Self::BLe { a, b, .. }
            | Self::BGe { a, b, .. } => {
                a.allocate_tmps(curr_loc, locals);
                b.allocate_tmps(curr_loc, locals);
            }
            Self::GoTo { .. } => (),
            Self::CallVirt { site: _, args } | Self::Call { site: _, args } => args
                .iter_mut()
                .for_each(|arg| arg.allocate_tmps(curr_loc, locals)),
            Self::SetField {
                addr,
                value,
                desc: _,
            } => {
                addr.allocate_tmps(curr_loc, locals);
                value.allocate_tmps(curr_loc, locals);
            }
            Self::CpBlk { src, dst, len } => {
                src.allocate_tmps(curr_loc, locals);
                dst.allocate_tmps(curr_loc, locals);
                len.allocate_tmps(curr_loc, locals);
            }
            Self::STIndI8(addr_calc, value_calc)
            | Self::STIndI16(addr_calc, value_calc)
            | Self::STIndI32(addr_calc, value_calc)
            | Self::STIndI64(addr_calc, value_calc)
            | Self::STIndISize(addr_calc, value_calc)
            | Self::STIndPtr(addr_calc, value_calc, _)
            | Self::STIndF64(addr_calc, value_calc)
            | Self::STIndF32(addr_calc, value_calc) => {
                addr_calc.allocate_tmps(curr_loc, locals);
                value_calc.allocate_tmps(curr_loc, locals);
            }
            Self::STObj {
                addr_calc,
                value_calc,
                ..
            } => {
                addr_calc.allocate_tmps(curr_loc, locals);
                value_calc.allocate_tmps(curr_loc, locals);
            }
            Self::STArg { arg: _, tree } => tree.allocate_tmps(curr_loc, locals),
            Self::Break => (),
            Self::Nop => (),
            Self::InitBlk { dst, val, count } => {
                dst.allocate_tmps(curr_loc, locals);
                val.allocate_tmps(curr_loc, locals);
                count.allocate_tmps(curr_loc, locals);
            }

            Self::Ret { tree } | Self::Pop { tree } | Self::Throw(tree) => {
                tree.allocate_tmps(curr_loc, locals);
            }
            Self::VoidRet => (),

            Self::ReThrow => (),
            Self::CallI {
                sig: _,
                fn_ptr,
                args,
            } => {
                fn_ptr.allocate_tmps(curr_loc, locals);
                args.iter_mut()
                    .for_each(|arg| arg.allocate_tmps(curr_loc, locals));
            }

            Self::SetTMPLocal { value } => {
                value.allocate_tmps(curr_loc, locals);
                *self = Self::STLoc {
                    local: curr_loc.expect("Referenced a tmp local when none present!"),
                    tree: value.clone(),
                };
            }
            Self::SetStaticField { descr: _, value } => value.allocate_tmps(curr_loc, locals),
            Self::JumpingPad { .. } => (),
        };
    }

    #[must_use]
    pub fn source_info(
        file: &str,
        line: std::ops::Range<u64>,
        column: std::ops::Range<u64>,
    ) -> Self {
        assert!(
            column.start < column.end,
            "PDB files must have columns that contain at least one element "
        );
        Self::SourceFileInfo(Box::new((line, column, file.to_owned().into())))
    }
    #[must_use]
    pub fn set_field(addr: CILNode, value: CILNode, desc: FieldDescriptor) -> Self {
        Self::SetField {
            addr: Box::new(addr),
            value: Box::new(value),
            desc: Box::new(desc),
        }
    }
}
fn tiny_message(msg: &str, asm: &mut Assembly) -> CILNode {
    let pieces: Vec<_> = msg.split_inclusive(char::is_whitespace).collect();
    runtime_string(&pieces, asm)
}
fn runtime_string(pieces: &[&str], asm: &mut Assembly) -> CILNode {
    match pieces.len() {
        0 => panic!("Incorrect piece count"),
        1 => CILNode::LdStr(pieces[0].to_owned().into()),
        2 => call!(
            CallSite::new_extern(
                ClassRef::string(asm),
                "Concat".to_owned().into(),
                FnSig::new(
                    Box::new([Type::PlatformString, Type::PlatformString,]),
                    Type::PlatformString,
                ),
                true
            ),
            [
                CILNode::LdStr(pieces[0].to_owned().into()),
                CILNode::LdStr(pieces[1].to_owned().into())
            ]
        ),
        3 => call!(
            CallSite::new_extern(
                ClassRef::string(asm),
                "Concat".to_owned().into(),
                FnSig::new(
                    Box::new([
                        Type::PlatformString,
                        Type::PlatformString,
                        Type::PlatformString,
                    ]),
                    Type::PlatformString,
                ),
                true
            ),
            [
                CILNode::LdStr(pieces[0].to_owned().into()),
                CILNode::LdStr(pieces[1].to_owned().into()),
                CILNode::LdStr(pieces[2].to_owned().into())
            ]
        ),
        4 => call!(
            CallSite::new_extern(
                ClassRef::string(asm),
                "Concat".to_owned().into(),
                FnSig::new(
                    Box::new([
                        Type::PlatformString,
                        Type::PlatformString,
                        Type::PlatformString,
                        Type::PlatformString,
                    ]),
                    Type::PlatformString,
                ),
                true
            ),
            [
                CILNode::LdStr(pieces[0].to_owned().into()),
                CILNode::LdStr(pieces[1].to_owned().into()),
                CILNode::LdStr(pieces[2].to_owned().into()),
                CILNode::LdStr(pieces[3].to_owned().into())
            ]
        ),
        _ => {
            let sub_part = pieces.len() / 4;
            call!(
                CallSite::new_extern(
                    ClassRef::string(asm),
                    "Concat".to_owned().into(),
                    FnSig::new(
                        Box::new([
                            Type::PlatformString,
                            Type::PlatformString,
                            Type::PlatformString,
                            Type::PlatformString,
                        ]),
                        Type::PlatformString,
                    ),
                    true
                ),
                [
                    runtime_string(&pieces[..sub_part], asm),
                    runtime_string(&pieces[sub_part..(sub_part * 2)], asm),
                    runtime_string(&pieces[(sub_part * 2)..(sub_part * 3)], asm),
                    runtime_string(&pieces[(sub_part * 3)..], asm)
                ]
            )
        }
    }
}
