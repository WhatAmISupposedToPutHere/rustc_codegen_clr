use std::any::type_name;

use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

use super::{
    bimap::{calculate_hash, BiMap},
    cilnode::{BinOp, MethodKind, UnOp},
    Access, CILNode, CILRoot, ClassDef, ClassDefIdx, ClassRef, ClassRefIdx, Const, FieldDesc,
    FieldIdx, FnSig, MethodDef, MethodDefIdx, MethodRef, MethodRefIdx, NodeIdx, RootIdx, SigIdx,
    StaticFieldDesc, StaticFieldIdx, StringIdx, Type, TypeIdx,
};
use crate::IString;
use crate::{asm::Assembly as V1Asm, v2::MethodImpl};
#[derive(Default, Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
struct IStringWrapper(IString);
impl std::hash::Hash for IStringWrapper {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for char in self.0.chars() {
            calculate_hash(&char).hash(state);
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
pub struct Assembly {
    strings: BiMap<StringIdx, IStringWrapper>,
    types: BiMap<TypeIdx, Type>,
    class_refs: BiMap<ClassRefIdx, ClassRef>,
    class_defs: FxHashMap<ClassDefIdx, ClassDef>,
    nodes: BiMap<NodeIdx, CILNode>,
    roots: BiMap<RootIdx, CILRoot>,
    sigs: BiMap<SigIdx, FnSig>,
    method_refs: BiMap<MethodRefIdx, MethodRef>,
    fields: BiMap<FieldIdx, FieldDesc>,
    statics: BiMap<StaticFieldIdx, StaticFieldDesc>,
    method_defs: FxHashMap<MethodRefIdx, MethodDef>,
}
impl Assembly {
    pub fn class_mut(&mut self, id: ClassDefIdx) -> &mut ClassDef {
        self.class_defs.get_mut(&id).unwrap()
    }
    pub fn alloc_string(&mut self, string: impl Into<IString>) -> StringIdx {
        self.strings.alloc(IStringWrapper(string.into()))
    }
    pub fn sig(&mut self, input: impl Into<Box<[Type]>>, output: impl Into<Type>) -> SigIdx {
        self.sigs.alloc(FnSig::new(input.into(), output.into()))
    }
    pub fn nptr(&mut self, inner: Type) -> Type {
        Type::Ptr(self.types.alloc(inner))
    }
    pub fn nref(&mut self, inner: Type) -> Type {
        Type::Ref(self.types.alloc(inner))
    }
    pub fn type_from_id(&self, idx: TypeIdx) -> &Type {
        self.types.get(idx)
    }
    pub fn biop(&mut self, lhs: impl Into<CILNode>, rhs: impl Into<CILNode>, op: BinOp) -> CILNode {
        let lhs = self.nodes.alloc(lhs.into());
        let rhs = self.nodes.alloc(rhs.into());
        CILNode::BinOp(lhs, rhs, op)
    }
    pub fn unop(&mut self, val: impl Into<CILNode>, op: UnOp) -> CILNode {
        let val = self.nodes.alloc(val.into());
        CILNode::UnOp(val, op)
    }
    pub fn ldstr(&mut self, msg: impl Into<IString>) -> CILNode {
        CILNode::Const(Const::PlatformString(self.alloc_string(msg)))
    }
    pub fn strct(&mut self, name: IString) -> ClassRefIdx {
        let class = ClassRef::new(self.alloc_string(name), None, true, vec![].into());
        self.class_refs.alloc(class)
    }

    pub(crate) fn node_idx(&mut self, node: CILNode) -> NodeIdx {
        self.nodes.alloc(node)
    }

    pub(crate) fn class_idx(&mut self, cref: ClassRef) -> ClassRefIdx {
        self.class_refs.alloc(cref)
    }

    pub(crate) fn sig_idx(&mut self, sig: FnSig) -> SigIdx {
        self.sigs.alloc(sig)
    }

    pub(crate) fn methodref_idx(&mut self, method_ref: MethodRef) -> MethodRefIdx {
        self.method_refs.alloc(method_ref)
    }

    pub(crate) fn alloc_root(&mut self, val: CILRoot) -> RootIdx {
        self.roots.alloc(val)
    }

    pub(crate) fn type_idx(&mut self, tpe: Type) -> TypeIdx {
        self.types.alloc(tpe)
    }

    pub(crate) fn get_node(&self, key: NodeIdx) -> &CILNode {
        self.nodes.get(key)
    }

    pub(crate) fn field_idx(&mut self, field: FieldDesc) -> FieldIdx {
        self.fields.alloc(field)
    }

    pub(crate) fn sfld_idx(&mut self, sfld: StaticFieldDesc) -> StaticFieldIdx {
        self.statics.alloc(sfld)
    }
    pub fn class_def(&mut self, def: ClassDef) -> ClassDefIdx {
        let cref = def.ref_to();
        let cref = self.class_idx(cref);

        if let Some(dup) = self.class_defs.insert(ClassDefIdx(cref), def.clone()) {
            panic!("duplicate class def. {dup:?} {def:?}");
        }

        ClassDefIdx(cref)
    }
    pub fn main_module(&mut self) -> ClassDefIdx {
        let main_module = self.alloc_string(MAIN_MODULE);

        let class_def = ClassDef::new(
            main_module,
            false,
            0,
            None,
            vec![],
            vec![],
            vec![],
            Access::Public,
            None,
        );
        let cref = class_def.ref_to();
        let cref = self.class_refs.alloc(cref);
        // Check if that definition already exists
        if self.class_defs.contains_key(&ClassDefIdx(cref)) {
            ClassDefIdx(cref)
        } else {
            self.class_def(class_def)
        }
    }
    /// Adds a method definition to this assembly.
    pub fn new_method(&mut self, def: MethodDef) -> MethodDefIdx {
        let mref = def.ref_to();
        let def_class = def.class();
        let ref_idx = self.methodref_idx(mref);
        self.class_defs
            .get_mut(&def_class)
            .expect("Method added without a class")
            .methods_mut()
            .push(MethodDefIdx(ref_idx));
        self.method_defs.insert(ref_idx, def);
        MethodDefIdx(ref_idx)
    }
    pub fn user_init(&mut self) -> MethodDefIdx {
        let main_module = self.main_module();
        let user_init = self.alloc_string(USER_INIT);
        let ctor_sig = self.sig([], Type::Void);
        let mref = MethodRef::new(
            Some(*main_module),
            user_init,
            ctor_sig,
            MethodKind::Static,
            vec![].into(),
        );
        let mref = self.methodref_idx(mref);
        match self.method_defs.entry(mref) {
            std::collections::hash_map::Entry::Occupied(_) => MethodDefIdx(mref),
            std::collections::hash_map::Entry::Vacant(_) => {
                let mimpl = MethodImpl::MethodBody {
                    blocks: vec![super::BasicBlock::new(
                        vec![self.alloc_root(CILRoot::VoidRet)],
                        0,
                        None,
                    )],
                    locals: vec![],
                };
                let cctor_def =
                    MethodDef::new(main_module, user_init, ctor_sig, MethodKind::Static, mimpl);
                self.new_method(cctor_def)
            }
        }
    }
    /// Adds new rooots to the user init list.
    pub fn add_user_init(&mut self, roots: &[RootIdx]) {
        let user_init = self.user_init();
        let user_init = self.method_defs.get_mut(&user_init).unwrap();
        let blocks = user_init
            .implementation_mut()
            .blocks_mut()
            .expect("EROROR: {USER_INIT} has no body.");
        let last = blocks
            .iter_mut()
            .last()
            .expect("ERROR: {USER_INIT} has a body without blocks.");
        let last_root_idx = if last.roots().is_empty() {
            0
        } else {
            last.roots().len() - 1
        };
        for root in roots {
            last.roots_mut().insert(last_root_idx, *root);
        }
    }
    /// Serializes and saves this assembly
    pub fn save_tmp<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&postcard::to_stdvec(&self).unwrap())
    }
    /// Converts the old assembly repr to the new one.
    pub fn from_v1(v1: &V1Asm) -> Self {
        let mut empty = Self::default();
        // Add the user defined roots
        let roots = v1
            .initializers()
            .iter()
            .map(|root| {
                let root = CILRoot::from_v1(root, &mut empty);
                empty.alloc_root(root)
            })
            .collect::<Box<[_]>>();
        empty.add_user_init(roots.as_ref());
        // Add the global static fields
        let fields: Vec<_> = v1
            .static_fields()
            .iter()
            .map(|(name, (tpe, thread_local))| {
                let tpe = Type::from_v1(tpe, &mut empty);
                let name = empty.alloc_string(name.clone());
                (tpe, name, *thread_local)
            })
            .collect();
        let main_module = empty.main_module();
        empty
            .class_defs
            .get_mut(&main_module)
            .expect("Main module missing, even tough it has been added")
            .static_fields_mut()
            .extend(fields);
        // Convert external function refs
        let extern_fns: Vec<_> = v1
            .extern_fns()
            .iter()
            .map(|((fn_name, sig, preserve_errno), lib_name)| {
                let sig = FnSig::from_v1(sig, &mut empty);
                MethodDef::new(
                    main_module,
                    empty.alloc_string(fn_name.clone()),
                    empty.sig_idx(sig),
                    MethodKind::Static,
                    MethodImpl::Extern {
                        lib: empty.alloc_string(lib_name.clone()),
                        preserve_errno: *preserve_errno,
                    },
                )
            })
            .collect();
        extern_fns.into_iter().for_each(|def| {
            empty.new_method(def);
        });
        // Convert module methods
        let fns: Vec<_> = v1
            .functions()
            .values()
            .map(|method| {
                let def = MethodDef::from_v1(method, &mut empty, main_module);
                empty.new_method(def)
            })
            .collect();
        empty
            .class_defs
            .get_mut(&main_module)
            .expect("Main module missing, even tough it has been added")
            .methods_mut()
            .extend(fns);
        //todo!();
        v1.types().for_each(|(_, tdef)| {
            ClassDef::from_v1(tdef, &mut empty);
        });
        empty
    }

    pub fn memory_info(&self) {
        encoded_stats(self);
        encoded_stats(&self.strings);
        encoded_stats(&self.types);
        encoded_stats(&self.class_refs);
        encoded_stats(&self.class_defs);
        encoded_stats(&self.nodes);
        encoded_stats(&self.roots);
        encoded_stats(&self.sigs);
        encoded_stats(&self.types);
        encoded_stats(&self.fields);
        encoded_stats(&self.statics);
        encoded_stats(&self.method_defs);
    }
}
/// An initializer, which runs before everything else. By convention, it is used to initialize static / const data. Should not execute any user code
pub const CCTOR: &str = ".cctor";
/// An thread-local initializer. Runs before each thread starts. By convention, it is used to initialize thread local data. Should not execute any user code.
pub const TCCTOR: &str = ".tcctor";
/// An intializer, which runs after the [`CCTOR`] and [`TCCTOR`], but before the [`ENTRYPOINT`]. Meant to execute user code, is roughly equivalnt to `.init_array` on GNU.
pub const USER_INIT: &str = "static_init";
/// The entrypoint of a program
pub const ENTRYPOINT: &str = "entrypoint";
/// Main class of this module
pub const MAIN_MODULE: &str = "MainModule";
fn encoded_stats<T: Serialize>(val: &T) {
    let buff = postcard::to_allocvec(val).unwrap();
    println!("{}:\t{} bytes", type_name::<T>(), buff.len());
}
#[test]
fn user_init() {
    let mut asm = Assembly::default();
    asm.user_init();
}
#[test]
fn add_user_init() {
    let mut asm = Assembly::default();
    let roots = vec![
        asm.alloc_root(CILRoot::VoidRet),
        asm.alloc_root(CILRoot::Break),
        asm.alloc_root(CILRoot::Nop),
    ];
    asm.add_user_init(&roots);
}
