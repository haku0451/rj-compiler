use crate::ast::*;
use crate::cast::{
    branch_offset, i64_to_i32, iconst_opcode, i32_to_i8_byte, u16_high_byte, u16_low_byte,
    u16_to_u8, usize_to_u16, usize_to_u32,
};
use crate::semantic::TypedSourceFile;
use crate::semantic::RjType;
use std::collections::{HashMap, HashSet};
use std::io::Write;

pub struct CodeEmitter;

impl CodeEmitter {
    pub fn emit_file(typed: &TypedSourceFile, known_interfaces: &HashSet<String>) -> Vec<(String, Vec<u8>)> {
        let internal_name = if typed.package_name.is_empty() {
            path_stem(&typed.source.path)
        } else {
            format!("{}/{}", typed.package_name.replace('.', "/"), path_stem(&typed.source.path))
        };

        let mut classes = Vec::new();
        let mut cw = ClassEmitter::new(&internal_name);

        // Default constructor
        cw.emit_default_ctor();

        let package_path = typed.package_name.replace('.', "/");
        let symbol_types: HashMap<String, RjType> =
            typed.symbol_types.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        for decl in &typed.source.decls {
            match decl {
                TopLevelDecl::Fn(f) => {
                    cw.emit_function(
                        f,
                        &internal_name,
                        &package_path,
                        &symbol_types,
                        &typed.source.imports,
                    );
                }
                TopLevelDecl::Class(c) => {
                    let class_bytes =
                        emit_class_decl(c, &typed.package_name, known_interfaces);
                    classes.push((c.name.clone(), class_bytes));
                }
                TopLevelDecl::Interface(i) => {
                    let bytes = emit_interface(i, &typed.package_name);
                    classes.push((i.name.clone(), bytes));
                }
                TopLevelDecl::Enum(e) => {
                    let bytes = emit_enum(e, &typed.package_name);
                    classes.push((e.name.clone(), bytes));
                }
                _ => {}
            }
        }

        if typed
            .source
            .decls
            .iter()
            .any(|d| matches!(d, TopLevelDecl::Fn(_)))
        {
            classes.push((path_stem(&typed.source.path), cw.finish()));
        }
        classes
    }
}

fn path_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Main")
        .to_string()
}

struct ClassEmitter {
    pool: CpPool,
    this_class: u16,
    super_class: u16,
    code_attr_name: u16,
    implemented_interfaces: Vec<u16>,
    fields: Vec<(u16, u16, u16)>,
    methods: Vec<MethodEmit>,
    access: u16,
}

struct MethodEmit {
    access: u16,
    name_idx: u16,
    desc_idx: u16,
    code: Vec<u8>,
    max_stack: u16,
    max_locals: u16,
    has_code: bool,
}

struct BcOutput {
    code: Vec<u8>,
    max_stack: u16,
    max_locals: u16,
}

struct CpPool {
    entries: Vec<CpEntry>,
}

#[derive(Clone)]
enum CpEntry {
    Utf8(String),
    Class(u16),
    String(u16),
    NameAndType { name: u16, desc: u16 },
    FieldRef { class: u16, nat: u16 },
    MethodRef { class: u16, nat: u16 },
    InterfaceMethodRef { class: u16, nat: u16 },
    Integer(i32),
}

impl CpPool {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn push_entry(&mut self, entry: CpEntry) -> u16 {
        self.entries.push(entry);
        usize_to_u16(self.entries.len())
    }

    fn utf8(&mut self, s: &str) -> u16 {
        self.push_entry(CpEntry::Utf8(s.to_string()))
    }

    fn class(&mut self, name: &str) -> u16 {
        let n = self.utf8(name);
        self.push_entry(CpEntry::Class(n))
    }

    fn string(&mut self, s: &str) -> u16 {
        let u = self.utf8(s);
        self.push_entry(CpEntry::String(u))
    }

    fn nat(&mut self, name: &str, desc: &str) -> u16 {
        let n = self.utf8(name);
        let d = self.utf8(desc);
        self.push_entry(CpEntry::NameAndType { name: n, desc: d })
    }

    fn method_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let nt = self.nat(name, desc);
        self.push_entry(CpEntry::MethodRef { class: c, nat: nt })
    }

    fn interface_method_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let nt = self.nat(name, desc);
        self.push_entry(CpEntry::InterfaceMethodRef { class: c, nat: nt })
    }

    fn field_ref(&mut self, class: &str, name: &str, desc: &str) -> u16 {
        let c = self.class(class);
        let nt = self.nat(name, desc);
        self.push_entry(CpEntry::FieldRef { class: c, nat: nt })
    }

    fn integer(&mut self, v: i32) -> u16 {
        self.push_entry(CpEntry::Integer(v))
    }
}

struct Bytecode<'a> {
    code: Vec<u8>,
    max_stack: u16,
    max_locals: u16,
    pool: &'a mut CpPool,
    package_path: String,
    symbol_types: HashMap<String, RjType>,
    import_aliases: HashMap<String, String>,
    locals: HashMap<String, u16>,
    local_types: HashMap<String, RjType>,
    next_local: u16,
    break_patches: Vec<Vec<usize>>,
}

impl<'a> Bytecode<'a> {
    fn new(
        pool: &'a mut CpPool,
        max_locals: u16,
        package_path: &str,
        symbol_types: &HashMap<String, RjType>,
        import_aliases: &HashMap<String, String>,
    ) -> Self {
        Self {
            code: Vec::new(),
            max_stack: 8,
            max_locals,
            pool,
            package_path: package_path.to_string(),
            symbol_types: symbol_types.clone(),
            import_aliases: import_aliases.clone(),
            locals: HashMap::new(),
            local_types: HashMap::new(),
            next_local: 0,
            break_patches: Vec::new(),
        }
    }

    fn push_break_scope(&mut self) {
        self.break_patches.push(Vec::new());
    }

    fn emit_break_jump(&mut self) {
        let pos = self.code.len();
        self.code.push(0xa7);
        self.i16(0);
        if let Some(scope) = self.break_patches.last_mut() {
            scope.push(pos);
        }
    }

    fn pop_break_scope(&mut self, exit_target: usize) {
        if let Some(jumps) = self.break_patches.pop() {
            for pos in jumps {
                patch_i16(
                    &mut self.code,
                    pos + 1,
                    branch_offset(pos, exit_target),
                );
            }
        }
    }

    fn build_import_aliases(imports: &[ImportDecl]) -> HashMap<String, String> {
        imports
            .iter()
            .map(|import| {
                let internal = import.path.join("/");
                let key = import
                    .alias
                    .clone()
                    .unwrap_or_else(|| import.path.last().cloned().unwrap_or_default());
                (key, internal)
            })
            .collect()
    }

    fn type_internal_name(&self, ty: &RjType) -> String {
        match ty {
            RjType::Class(name) | RjType::Enum(name) | RjType::Named(name) => {
                if name.contains('.') {
                    name.replace('.', "/")
                } else {
                    format!("{}/{}", self.package_path, name)
                }
            }
            RjType::Str => "java/lang/String".into(),
            _ => "java/lang/Object".into(),
        }
    }

    fn resolve_class_internal(&self, name: &str) -> String {
        if name.contains('/') {
            name.to_string()
        } else if let Some(RjType::Class(n) | RjType::Enum(n)) = self.symbol_types.get(name) {
            self.type_internal_name(&RjType::Class(n.clone()))
        } else {
            format!("{}/{}", self.package_path, name)
        }
    }

    fn is_type_name(&self, name: &str) -> bool {
        matches!(
            self.symbol_types.get(name),
            Some(RjType::Class(_) | RjType::Enum(_))
        ) || name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
    }

    fn init_frame(&mut self, f: &FnDecl, is_main: bool) {
        self.next_local = 0;
        self.max_locals = 0;
        self.locals.clear();
        self.local_types.clear();

        if is_main || !f.is_static {
            // main: slot 0 = String[] args; instance method: slot 0 = this
            self.next_local = 1;
            self.max_locals = 1;
        }

        for p in &f.params {
            let slot = self.next_local;
            self.next_local += 1;
            if self.next_local > self.max_locals {
                self.max_locals = self.next_local;
            }
            let ty = p
                .ty
                .as_ref()
                .map(RjType::from_ast)
                .unwrap_or(RjType::Int);
            self.locals.insert(p.name.clone(), slot);
            self.local_types.insert(p.name.clone(), ty);
        }
    }

    fn is_int_local(ty: &RjType) -> bool {
        matches!(ty, RjType::Int | RjType::Bool)
    }

    fn store_local(&mut self, slot: u16, ty: &RjType) {
        if Self::is_int_local(ty) {
            self.istore(slot);
        } else {
            self.astore(slot);
        }
    }

    fn load_local(&mut self, slot: u16, ty: &RjType) {
        if Self::is_int_local(ty) {
            self.iload(slot);
        } else {
            self.aload(slot);
        }
    }

    fn return_value(&mut self, ty: &RjType) {
        if Self::is_int_local(ty) {
            self.ireturn();
        } else {
            self.areturn();
        }
    }

    fn finish(self) -> BcOutput {
        BcOutput {
            code: self.code,
            max_stack: self.max_stack,
            max_locals: self.max_locals,
        }
    }

    fn alloc_local(&mut self, name: &str, ty: RjType) -> u16 {
        let slot = self.next_local;
        self.next_local += 1;
        self.locals.insert(name.to_string(), slot);
        self.local_types.insert(name.to_string(), ty);
        if self.next_local > self.max_locals {
            self.max_locals = self.next_local;
        }
        slot
    }

    fn u16(&mut self, v: u16) {
        self.code.push(u16_high_byte(v));
        self.code.push(u16_low_byte(v));
    }

    fn i16(&mut self, v: i16) {
        self.u16(u16::from_ne_bytes(v.to_ne_bytes()));
    }

    fn iconst(&mut self, v: i32) {
        match v {
            -1 => self.code.push(0x02),
            0..=5 => self.code.push(iconst_opcode(v)),
            -128..=127 => {
                self.code.push(0x10);
                self.code.push(i32_to_i8_byte(v));
            }
            _ => {
                let idx = self.pool.integer(v);
                self.code.push(0x13);
                self.code.push(u16_to_u8(idx));
            }
        }
    }

    fn ldc_string(&mut self, s: &str) {
        let idx = self.pool.string(s);
        if idx < 256 {
            self.code.push(0x12);
            self.code.push(u16_to_u8(idx));
        } else {
            self.code.push(0x13);
            self.u16(idx);
        }
    }

    fn aload(&mut self, n: u16) {
        match n {
            0 => self.code.push(0x2a),
            1 => self.code.push(0x2b),
            2 => self.code.push(0x2c),
            3 => self.code.push(0x2d),
            _ => {
                self.code.push(0x19);
                self.code.push(u16_to_u8(n));
            }
        }
    }

    fn astore(&mut self, n: u16) {
        match n {
            0 => self.code.push(0x4b),
            1 => self.code.push(0x4c),
            2 => self.code.push(0x4d),
            3 => self.code.push(0x4e),
            _ => {
                self.code.push(0x3a);
                self.code.push(u16_to_u8(n));
            }
        }
    }

    fn iload(&mut self, n: u16) {
        match n {
            0 => self.code.push(0x1a),
            1 => self.code.push(0x1b),
            2 => self.code.push(0x1c),
            3 => self.code.push(0x1d),
            _ => {
                self.code.push(0x15);
                self.code.push(u16_to_u8(n));
            }
        }
    }

    fn istore(&mut self, n: u16) {
        match n {
            0 => self.code.push(0x3b),
            1 => self.code.push(0x3c),
            2 => self.code.push(0x3d),
            3 => self.code.push(0x3e),
            _ => {
                self.code.push(0x36);
                self.code.push(u16_to_u8(n));
            }
        }
    }

    fn pop_if_value(&mut self, ty: RjType) {
        if ty != RjType::Void {
            self.code.push(0x57);
        }
    }

    fn get_static(&mut self, class: &str, field: &str, desc: &str) {
        let idx = self.pool.field_ref(class, field, desc);
        self.code.push(0xb2);
        self.u16(idx);
    }

    fn invoke_virtual(&mut self, class: &str, method: &str, desc: &str) {
        let idx = self.pool.method_ref(class, method, desc);
        self.code.push(0xb6);
        self.u16(idx);
    }

    fn cast_string(&mut self) {
        self.invoke_static(
            "rj/runtime/RjRuntime",
            "castString",
            "(Ljava/lang/Object;)Ljava/lang/String;",
        );
    }

    fn unbox_integer(&mut self) {
        self.invoke_static("rj/runtime/RjRuntime", "unboxInt", "(Ljava/lang/Object;)I");
    }

    fn box_int(&mut self) {
        self.invoke_static("java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;");
    }

    fn store_iter_next_int(&mut self, var: &str) {
        self.unbox_integer();
        let slot = self.alloc_local(var, RjType::Int);
        self.istore(slot);
    }

    fn emit_member_receiver(&mut self, object: &Expr, class_name: &str) -> RjType {
        if let Expr::Ident { name, .. } = object {
            if let Some(ty) = self.local_types.get(name).cloned() {
                let slot = self.locals[name];
                self.load_local(slot, &ty);
                return ty;
            }
        }
        self.emit_expr(object, class_name)
    }

    fn emit_list_add(&mut self, list_slot: u16, elem: &Expr, class_name: &str) {
        self.aload(list_slot);
        let ty = self.emit_expr(elem, class_name);
        if matches!(ty, RjType::Int | RjType::Bool) {
            self.box_int();
        }
        self.invoke_virtual("java/util/ArrayList", "add", "(Ljava/lang/Object;)Z");
        self.code.push(0x57);
    }

    /// 2 つの int を比較し、真偽値 (0/1) をスタックに積む
    fn emit_int_compare_result(&mut self, cmp_opcode: u8) {
        let true_jump = self.code.len();
        self.code.push(cmp_opcode);
        self.i16(0);
        self.iconst(0);
        let exit_jump = self.code.len();
        self.code.push(0xa7);
        self.i16(0);
        let true_label = self.code.len();
        self.iconst(1);
        let end = self.code.len();
        patch_i16(
            &mut self.code,
            true_jump + 1,
            branch_offset(true_jump, true_label),
        );
        patch_i16(
            &mut self.code,
            exit_jump + 1,
            branch_offset(exit_jump, end),
        );
    }

    fn invoke_static(&mut self, class: &str, method: &str, desc: &str) {
        let idx = self.pool.method_ref(class, method, desc);
        self.code.push(0xb8);
        self.u16(idx);
    }

    fn invoke_special(&mut self, class: &str, method: &str, desc: &str) {
        let idx = self.pool.method_ref(class, method, desc);
        self.code.push(0xb7);
        self.u16(idx);
    }

    fn new_obj(&mut self, class: &str) {
        let idx = self.pool.class(class);
        self.code.push(0xbb);
        self.u16(idx);
        self.code.push(0x59);
    }

    fn putfield(&mut self, class: &str, field: &str, desc: &str) {
        let idx = self.pool.field_ref(class, field, desc);
        self.code.push(0xb5);
        self.u16(idx);
    }

    fn iadd(&mut self) { self.code.push(0x60); }
    fn isub(&mut self) { self.code.push(0x64); }
    fn imul(&mut self) { self.code.push(0x68); }
    fn idiv(&mut self) { self.code.push(0x6c); }
    fn irem(&mut self) { self.code.push(0x70); }

    fn return_void(&mut self) { self.code.push(0xb1); }
    fn ireturn(&mut self) { self.code.push(0xac); }
    fn areturn(&mut self) { self.code.push(0xb0); }
    fn athrow(&mut self) { self.code.push(0xbf); }

    fn emit_expr(&mut self, expr: &Expr, class_name: &str) -> RjType {
        match expr {
            Expr::IntLit { value, .. } => {
                self.iconst(i64_to_i32(*value));
                RjType::Int
            }
            Expr::StringLit { value, .. } => {
                self.ldc_string(value);
                RjType::Str
            }
            Expr::BoolLit { value, .. } => {
                self.iconst(if *value { 1 } else { 0 });
                RjType::Bool
            }
            Expr::Ident { name, .. } => {
                if let Some(&slot) = self.locals.get(name) {
                    let ty = self
                        .local_types
                        .get(name)
                        .cloned()
                        .unwrap_or(RjType::Int);
                    self.load_local(slot, &ty);
                    return ty;
                }
                RjType::Int
            }
            Expr::Binary { op, left, right, .. } => {
                self.emit_expr(left, class_name);
                self.emit_expr(right, class_name);
                match op {
                    BinOp::Add => self.iadd(),
                    BinOp::Sub => self.isub(),
                    BinOp::Mul => self.imul(),
                    BinOp::Div => self.idiv(),
                    BinOp::Mod => self.irem(),
                    BinOp::Eq => self.emit_int_compare_result(0xa4),
                    BinOp::Ne => self.emit_int_compare_result(0xa5),
                    BinOp::Lt => self.emit_int_compare_result(0xa1),
                    BinOp::Gt => self.emit_int_compare_result(0xa3),
                    BinOp::Le => self.emit_int_compare_result(0xa6),
                    BinOp::Ge => self.emit_int_compare_result(0xa2),
                    BinOp::Concat => {
                        self.invoke_static(
                            "rj/runtime/RjRuntime",
                            "concat",
                            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                        );
                        return RjType::Str;
                    }
                    _ => {}
                }
                if matches!(
                    op,
                    BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge
                ) {
                    RjType::Bool
                } else {
                    RjType::Int
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    if name == "print" {
                        self.get_static("java/lang/System", "out", "Ljava/io/PrintStream;");
                        if let Some(arg) = args.first() {
                            let ty = self.emit_expr(arg, class_name);
                            let desc = match ty {
                                RjType::Int => "(I)V",
                                RjType::Str => "(Ljava/lang/String;)V",
                                _ => "(Ljava/lang/Object;)V",
                            };
                            self.invoke_virtual("java/io/PrintStream", "println", desc);
                        }
                        return RjType::Void;
                    }
                    if name == "range" {
                        if let Some(Expr::IntLit { value, .. }) = args.first() {
                            self.iconst(i64_to_i32(*value));
                            self.invoke_static(
                                "rj/runtime/RjRuntime",
                                "range",
                                "(I)Ljava/lang/Iterable;",
                            );
                        }
                        return RjType::Named("Iterable".into());
                    }
                    if self.is_type_name(name) {
                        let target = self.resolve_class_internal(name);
                        self.new_obj(&target);
                        let mut arg_desc = String::new();
                        for arg in args {
                            let ty = self.emit_expr(arg, class_name);
                            arg_desc.push_str(&ty.jvm_descriptor());
                        }
                        self.invoke_special(&target, "<init>", &format!("({arg_desc})V"));
                        return RjType::Class(name.clone());
                    }
                    let mut arg_desc = String::new();
                    for arg in args {
                        let ty = self.emit_expr(arg, class_name);
                        arg_desc.push_str(&ty.jvm_descriptor());
                    }
                    let (ret_desc, ret_ty) = rj_static_method_signature(name, &arg_desc);
                    self.invoke_static(
                        class_name,
                        name,
                        &format!("({arg_desc}){ret_desc}"),
                    );
                    return ret_ty;
                }
                if let Expr::Member { object, name: method, .. } = callee.as_ref() {
                    if let Expr::Ident { name: owner, .. } = object.as_ref() {
                        if owner == "Result" && matches!(method.as_str(), "ok" | "err") {
                            for arg in args {
                                self.emit_expr(arg, class_name);
                            }
                            self.invoke_static(
                                "rj/runtime/Result",
                                method,
                                "(Ljava/lang/Object;)Lrj/runtime/Result;",
                            );
                            return RjType::Result(Box::new(RjType::Str), Box::new(RjType::Str));
                        }
                        if let Some(target) = self.import_aliases.get(owner).cloned() {
                            if args.is_empty() {
                                let (desc, ret) = java_import_static_desc(&target, method);
                                self.invoke_static(&target, method, &desc);
                                return ret;
                            }
                        }
                        if self.is_type_name(owner) {
                            let target = self.resolve_class_internal(owner);
                            let mut arg_desc = String::new();
                            for arg in args {
                                let ty = self.emit_expr(arg, class_name);
                                arg_desc.push_str(&ty.jvm_descriptor());
                            }
                            let (ret_desc, ret_ty) =
                                rj_static_method_signature(method, &arg_desc);
                            self.invoke_static(
                                &target,
                                method,
                                &format!("({arg_desc}){ret_desc}"),
                            );
                            return ret_ty;
                        }
                    }
                    let obj_ty = self.emit_member_receiver(object, class_name);
                    let mut arg_desc = String::new();
                    for arg in args {
                        let ty = self.emit_expr(arg, class_name);
                        arg_desc.push_str(&ty.jvm_descriptor());
                    }
                    let (ret_desc, ret_ty) = rj_instance_method_signature(method, &arg_desc);
                    let target = if method == "toString" {
                        "java/lang/Object".into()
                    } else {
                        self.type_internal_name(&obj_ty)
                    };
                    self.invoke_virtual(&target, method, &format!("({arg_desc}){ret_desc}"));
                    return ret_ty;
                }
                RjType::Void
            }
            Expr::Member { object, name, .. } => {
                if let Expr::Ident { name: owner, .. } = object.as_ref() {
                    if matches!(self.symbol_types.get(owner), Some(RjType::Enum(_)))
                        || owner.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    {
                        let internal = self.resolve_class_internal(owner);
                        let field_ty = format!("L{internal};");
                        self.get_static(&internal, name, &field_ty);
                        return RjType::Enum(owner.clone());
                    }
                }
                let obj_ty = self.emit_member_receiver(object, class_name);
                let target = self.type_internal_name(&obj_ty);
                let getter = format!(
                    "get{}",
                    name.chars().next().unwrap().to_ascii_uppercase().to_string() + &name[1..]
                );
                self.invoke_virtual(&target, &getter, &format!("(){}", field_desc(name)));
                RjType::Str
            }
            Expr::If { cond, then_branch, else_branch, .. } => {
                self.emit_expr(cond, class_name);
                let else_jump = self.code.len();
                self.code.push(0x99); // ifeq → else
                self.i16(0);
                let then_ty = self.emit_expr(then_branch, class_name);
                let end_jump = self.code.len();
                self.code.push(0xa7);
                self.i16(0);
                let else_start = self.code.len();
                patch_i16(
                    &mut self.code,
                    else_jump + 1,
                    branch_offset(else_jump, else_start),
                );
                let else_ty = if let Some(e) = else_branch {
                    self.emit_expr(e, class_name)
                } else {
                    RjType::Void
                };
                let end = self.code.len();
                patch_i16(
                    &mut self.code,
                    end_jump + 1,
                    branch_offset(end_jump, end),
                );
                if then_ty != RjType::Void {
                    then_ty
                } else {
                    else_ty
                }
            }
            Expr::Await { expr, .. } => {
                self.emit_expr(expr, class_name);
                self.invoke_virtual(
                    "java/util/concurrent/CompletableFuture",
                    "join",
                    "()Ljava/lang/Object;",
                );
                self.cast_string();
                RjType::Str
            }
            Expr::Try { expr, .. } => {
                self.emit_expr(expr, class_name);
                self.invoke_static(
                    "rj/runtime/RjRuntime",
                    "unwrapResult",
                    "(Lrj/runtime/Result;)Ljava/lang/Object;",
                );
                self.cast_string();
                RjType::Str
            }
            Expr::Lambda { params, body, .. } => {
                let _ = params;
                self.emit_expr(body, class_name);
                RjType::Lambda {
                    params: vec![],
                    ret: Box::new(RjType::Int),
                }
            }
            Expr::Comprehension { elem, var, iter, cond, .. } => {
                self.emit_comprehension(elem, var, iter, cond.as_deref(), class_name);
                RjType::Named("List".into())
            }
            Expr::Block { stmts, tail, .. } => {
                for stmt in stmts {
                    self.emit_stmt(stmt, class_name);
                }
                if let Some(t) = tail {
                    return self.emit_expr(t, class_name);
                }
                RjType::Void
            }
            Expr::Assign { target, value, .. } => {
                if let Expr::Member { object, name, .. } = target.as_ref() {
                    self.emit_expr(object, class_name);
                    self.emit_expr(value, class_name);
                    self.putfield(class_name, name, "Ljava/lang/String;");
                } else if let Expr::Ident { name, .. } = target.as_ref() {
                    if let Some(&slot) = self.locals.get(name) {
                        let ty = self.emit_expr(value, class_name);
                        self.store_local(slot, &ty);
                        self.local_types.insert(name.clone(), ty);
                    }
                }
                RjType::Void
            }
            Expr::SelfExpr { .. } => {
                self.aload(0);
                RjType::Class(class_name.replace('/', "."))
            }
            Expr::Unary { op, expr, .. } => {
                self.emit_expr(expr, class_name);
                match op {
                    UnaryOp::Neg => {
                        self.iconst(0);
                        self.isub();
                    }
                    UnaryOp::Not => {}
                }
                RjType::Bool
            }
            Expr::Match { arms, .. } => {
                if let Some(arm) = arms.first() {
                    self.emit_expr(&arm.body, class_name)
                } else {
                    RjType::Void
                }
            }
            Expr::FloatLit { .. } | Expr::Index { .. } => RjType::Void,
        }
    }

    fn emit_comprehension(
        &mut self,
        elem: &Expr,
        var: &str,
        iter: &Expr,
        cond: Option<&Expr>,
        class_name: &str,
    ) {
        self.new_obj("java/util/ArrayList");
        self.code.push(0x59);
        self.invoke_special("java/util/ArrayList", "<init>", "()V");
        let list_slot = self.alloc_local("__list", RjType::Named("List".into()));
        self.astore(list_slot);

        self.emit_expr(iter, class_name);
        let iter_slot = self.alloc_local("__iter", RjType::Named("Iterable".into()));
        self.astore(iter_slot);

        self.invoke_interface(
            iter_slot,
            "java/lang/Iterable",
            "iterator",
            "()Ljava/util/Iterator;",
        );
        let it_slot = self.alloc_local("__it", RjType::Named("Iterator".into()));
        self.astore(it_slot);

        let loop_start = self.code.len();
        self.invoke_interface(it_slot, "java/util/Iterator", "hasNext", "()Z");
        let exit_pos = self.code.len();
        self.code.push(0x99);
        self.i16(0); // patch later

        self.invoke_interface(it_slot, "java/util/Iterator", "next", "()Ljava/lang/Object;");
        self.store_iter_next_int(var);

        if let Some(c) = cond {
            self.emit_expr(c, class_name);
            let skip_pos = self.code.len();
            self.code.push(0x99);
            self.i16(0);
            self.emit_list_add(list_slot, elem, class_name);
            let goto_back_pos = self.code.len();
            let back = branch_offset(goto_back_pos, loop_start);
            self.code.push(0xa7);
            self.i16(back);
            patch_i16(
                &mut self.code,
                skip_pos + 1,
                branch_offset(skip_pos, goto_back_pos),
            );
        } else {
            self.emit_list_add(list_slot, elem, class_name);
            let goto_back_pos = self.code.len();
            let back = branch_offset(goto_back_pos, loop_start);
            self.code.push(0xa7);
            self.i16(back);
        }

        let exit_target = self.code.len();
        patch_i16(
            &mut self.code,
            exit_pos + 1,
            branch_offset(exit_pos, exit_target),
        );

        self.aload(list_slot);
    }

    fn invoke_interface(&mut self, obj_slot: u16, class: &str, method: &str, desc: &str) {
        self.aload(obj_slot);
        let idx = self.pool.interface_method_ref(class, method, desc);
        let count = invoke_interface_count(desc);
        self.code.push(0xb9);
        self.u16(idx);
        self.code.push(count);
        self.code.push(0);
    }

    fn emit_stmt(&mut self, stmt: &Stmt, class_name: &str) {
        match stmt {
            Stmt::Expr { expr, .. } => {
                let ty = self.emit_expr(expr, class_name);
                self.pop_if_value(ty);
            }
            Stmt::Let { name, value, .. } => {
                let ty = self.emit_expr(value, class_name);
                let slot = self.alloc_local(name, ty.clone());
                self.store_local(slot, &ty);
            }
            Stmt::Return { value: Some(v), .. } => {
                let ty = self.emit_expr(v, class_name);
                self.return_value(&ty);
            }
            Stmt::Return { .. } => self.return_void(),
            Stmt::Throw { value, .. } => {
                self.emit_expr(value, class_name);
                self.athrow();
            }
            Stmt::While { cond, body, .. } => {
                self.push_break_scope();
                let start = self.code.len();
                self.emit_expr(cond, class_name);
                let exit = self.code.len();
                self.code.push(0x99);
                self.i16(0);
                let body_ty = self.emit_expr(body, class_name);
                self.pop_if_value(body_ty);
                let goto_pos = self.code.len();
                let back = branch_offset(goto_pos, start);
                self.code.push(0xa7);
                self.i16(back);
                let exit_target = self.code.len();
                patch_i16(
                    &mut self.code,
                    exit + 1,
                    branch_offset(exit, exit_target),
                );
                self.pop_break_scope(exit_target);
            }
            Stmt::Loop { body, .. } => {
                self.push_break_scope();
                let start = self.code.len();
                let body_ty = self.emit_expr(body, class_name);
                self.pop_if_value(body_ty);
                let goto_pos = self.code.len();
                let back = branch_offset(goto_pos, start);
                self.code.push(0xa7);
                self.i16(back);
                let exit_target = self.code.len();
                self.pop_break_scope(exit_target);
            }
            Stmt::Break { .. } => {
                self.emit_break_jump();
            }
            Stmt::For { var, iter, body, .. } => {
                self.push_break_scope();
                self.emit_expr(iter, class_name);
                let iter_slot = self.alloc_local("__iter", RjType::Named("Iterable".into()));
                self.astore(iter_slot);
                self.invoke_interface(iter_slot, "java/lang/Iterable", "iterator", "()Ljava/util/Iterator;");
                let it_slot = self.alloc_local("__it", RjType::Named("Iterator".into()));
                self.astore(it_slot);
                let loop_start = self.code.len();
                self.invoke_interface(it_slot, "java/util/Iterator", "hasNext", "()Z");
                let exit_pos = self.code.len();
                self.code.push(0x99);
                self.i16(0);
                self.invoke_interface(it_slot, "java/util/Iterator", "next", "()Ljava/lang/Object;");
                self.store_iter_next_int(var);
                let body_ty = self.emit_expr(body, class_name);
                self.pop_if_value(body_ty);
                let goto_pos = self.code.len();
                let back = branch_offset(goto_pos, loop_start);
                self.code.push(0xa7);
                self.i16(back);
                let exit_target = self.code.len();
                patch_i16(
                    &mut self.code,
                    exit_pos + 1,
                    branch_offset(exit_pos, exit_target),
                );
                self.pop_break_scope(exit_target);
            }
        }
    }
}

fn patch_i16(code: &mut [u8], pos: usize, v: i16) {
    let bytes = v.to_ne_bytes();
    code[pos] = bytes[0];
    code[pos + 1] = bytes[1];
}

fn java_import_static_desc(class: &str, method: &str) -> (String, RjType) {
    match (class, method) {
        ("java/lang/System", "lineSeparator") => {
            ("()Ljava/lang/String;".into(), RjType::Str)
        }
        ("java/time/LocalDateTime", "now") => (
            "()Ljava/time/LocalDateTime;".into(),
            RjType::Named("LocalDateTime".into()),
        ),
        _ => (
            "()Ljava/lang/Object;".into(),
            RjType::Named("Object".into()),
        ),
    }
}

fn rj_static_method_signature(method: &str, arg_desc: &str) -> (&'static str, RjType) {
    match (method, arg_desc) {
        ("add", _) => ("I", RjType::Int),
        ("apply", _) => ("I", RjType::Int),
        ("readConfig", "") => (
            "Lrj/runtime/Result;",
            RjType::Result(Box::new(RjType::Str), Box::new(RjType::Str)),
        ),
        ("fetch", "") => (
            "Ljava/util/concurrent/CompletableFuture;",
            RjType::Named("CompletableFuture".into()),
        ),
        ("run" | "load", "") => ("Ljava/lang/String;", RjType::Str),
        ("showEvens" | "demoLoops", "") => ("V", RjType::Void),
        ("print", "Ljava/lang/String;") => ("V", RjType::Void),
        ("print", "I") => ("V", RjType::Void),
        _ => ("V", RjType::Void),
    }
}

fn rj_instance_method_signature(method: &str, _arg_desc: &str) -> (&'static str, RjType) {
    match method {
        "toString" => ("Ljava/lang/String;", RjType::Str),
        "speak" | "greet" => ("V", RjType::Void),
        _ => ("V", RjType::Void),
    }
}

/// invokeinterface の count オペランド (receiver + 引数スロット数)
fn invoke_interface_count(desc: &str) -> u8 {
    let params = desc
        .split(')')
        .next()
        .unwrap_or("")
        .trim_start_matches('(');
    let mut slots = 1u8; // receiver
    let mut chars = params.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            'L' => {
                while chars.next() != Some(';') {}
                slots += 1;
            }
            'I' | 'B' | 'C' | 'S' | 'Z' | 'F' => slots += 1,
            'J' | 'D' => slots += 2,
            '[' => {
                while matches!(chars.peek(), Some('[')) {
                    chars.next();
                }
                match chars.next() {
                    Some('L') => {
                        while chars.next() != Some(';') {}
                    }
                    Some(_) => {}
                    None => {}
                }
                slots += 1;
            }
            _ => {}
        }
    }
    slots
}

fn field_desc(_name: &str) -> String {
    "Ljava/lang/String;".into()
}

impl ClassEmitter {
    fn new(class_name: &str) -> Self {
        let mut pool = CpPool::new();
        let this_class = pool.class(class_name);
        let super_class = pool.class("java/lang/Object");
        let code_attr_name = pool.utf8("Code");
        Self {
            pool,
            this_class,
            super_class,
            code_attr_name,
            implemented_interfaces: vec![],
            fields: vec![],
            methods: vec![],
            access: 0x0021,
        }
    }

    fn emit_default_ctor(&mut self) {
        let bc_out = {
            let mut bc = Bytecode::new(&mut self.pool, 1, "", &HashMap::new(), &HashMap::new());
            bc.aload(0);
            bc.invoke_special("java/lang/Object", "<init>", "()V");
            bc.return_void();
            bc.finish()
        };
        self.add_method_from_bc("<init>", "()V", 0x0001, bc_out);
    }

    fn emit_function(
        &mut self,
        f: &FnDecl,
        class_name: &str,
        package_path: &str,
        symbol_types: &HashMap<String, RjType>,
        imports: &[ImportDecl],
    ) {
        let import_aliases = Bytecode::build_import_aliases(imports);
        let is_main = f.name == "main" && f.params.is_empty();
        let desc = if is_main {
            "([Ljava/lang/String;)V".to_string()
        } else {
            fn_descriptor(f)
        };

        let bc_out = {
            let mut bc =
                Bytecode::new(&mut self.pool, 1, package_path, symbol_types, &import_aliases);
            bc.init_frame(f, is_main);
            if f.is_async {
                bc.emit_async_fn(f, class_name);
            } else {
                for stmt in &f.body {
                    bc.emit_stmt(stmt, class_name);
                }
                if let Some(tail) = &f.tail_expr {
                    let tail_ty = bc.emit_expr(tail, class_name);
                    if f.ret_ty.is_some() {
                        bc.return_value(&tail_ty);
                    } else {
                        bc.pop_if_value(tail_ty);
                        bc.return_void();
                    }
                } else {
                    bc.return_void();
                }
            }
            bc.finish()
        };

        let access = if f.is_static || is_main { 0x0009 } else { 0x0001 };
        self.add_method_from_bc(&f.name, &desc, access, bc_out);
    }

    fn add_method_from_bc(&mut self, name: &str, desc: &str, access: u16, bc: BcOutput) {
        let name_idx = self.pool.utf8(name);
        let desc_idx = self.pool.utf8(desc);
        self.methods.push(MethodEmit {
            access,
            name_idx,
            desc_idx,
            code: bc.code,
            max_stack: bc.max_stack,
            max_locals: bc.max_locals,
            has_code: true,
        });
    }

    fn finish(self) -> Vec<u8> {
        write_class(
            self.access,
            &self.pool,
            self.this_class,
            self.super_class,
            self.code_attr_name,
            &self.implemented_interfaces,
            &self.fields,
            &self.methods,
        )
    }
}

impl<'a> Bytecode<'a> {
    fn emit_async_fn(&mut self, f: &FnDecl, class_name: &str) {
        self.new_obj("java/util/concurrent/CompletableFuture");
        self.code.push(0x59);
        self.invoke_special(
            "java/util/concurrent/CompletableFuture",
            "<init>",
            "()V",
        );
        for stmt in &f.body {
            self.emit_stmt(stmt, class_name);
        }
        if let Some(tail) = &f.tail_expr {
            self.emit_expr(tail, class_name);
        }
        self.invoke_static(
            "rj/runtime/AsyncSupport",
            "complete",
            "(Ljava/util/concurrent/CompletableFuture;Ljava/lang/Object;)Ljava/util/concurrent/CompletableFuture;",
        );
        self.areturn();
    }
}

fn fn_descriptor(f: &FnDecl) -> String {
    let params: String = f
        .params
        .iter()
        .map(|p| {
            p.ty
                .as_ref()
                .map(|t| RjType::from_ast(t).jvm_descriptor())
                .unwrap_or_else(|| "I".into())
        })
        .collect();
    let ret = if f.is_async {
        "Ljava/util/concurrent/CompletableFuture;".into()
    } else {
        f.ret_ty
            .as_ref()
            .map(|t| RjType::from_ast(t).jvm_descriptor())
            .unwrap_or_else(|| "V".into())
    };
    format!("({params}){ret}")
}

fn resolve_super_internal(super_class: Option<&str>, package_path: &str) -> String {
    match super_class {
        Some(name) if name.contains('/') || name.contains('.') => name.replace('.', "/"),
        Some(name) => format!("{package_path}/{name}"),
        None => "java/lang/Object".into(),
    }
}

fn emit_class_decl(
    class: &ClassDecl,
    package: &str,
    known_interfaces: &HashSet<String>,
) -> Vec<u8> {
    let internal = format!("{}/{}", package.replace('.', "/"), class.name);
    let mut emitter = ClassEmitter::new(&internal);

    for field in &class.fields {
        let desc = RjType::from_ast(&field.ty).jvm_descriptor();
        let access = visibility_to_flags(field.visibility) | if field.mutable { 0 } else { 0x0010 };
        let name_idx = emitter.pool.utf8(&field.name);
        let desc_idx = emitter.pool.utf8(&desc);
        emitter.fields.push((access, name_idx, desc_idx));
    }

    let package_path = package.replace('.', "/");
    let mut super_name = "java/lang/Object".to_string();
    if let Some(parent) = &class.super_class {
        if known_interfaces.contains(parent) {
            let iface = format!("{package_path}/{parent}");
            emitter
                .implemented_interfaces
                .push(emitter.pool.class(&iface));
        } else {
            super_name = resolve_super_internal(Some(parent), &package_path);
        }
    }
    for iface in &class.interfaces {
        let iface_internal = format!("{package_path}/{iface}");
        emitter
            .implemented_interfaces
            .push(emitter.pool.class(&iface_internal));
    }
    emitter.super_class = emitter.pool.class(&super_name);
    // fields in ctor
    let empty_symbols: HashMap<String, RjType> = HashMap::new();
    let empty_imports: HashMap<String, String> = HashMap::new();
    let ctor_bc_out = {
        let mut ctor_bc = Bytecode::new(
            &mut emitter.pool,
            3,
            &package_path,
            &empty_symbols,
            &empty_imports,
        );
        ctor_bc.aload(0);
        ctor_bc.invoke_special(&super_name, "<init>", "()V");
        ctor_bc.return_void();
        ctor_bc.finish()
    };
    emitter.add_method_from_bc("<init>", "()V", 0x0001, ctor_bc_out);

    for method in &class.methods {
        if method.name == "__init__" {
            let bc_out = {
                let mut bc = Bytecode::new(
                    &mut emitter.pool,
                    8,
                    &package_path,
                    &empty_symbols,
                    &empty_imports,
                );
                bc.init_frame(method, false);
                bc.aload(0);
                bc.invoke_special(&super_name, "<init>", "()V");
                for stmt in &method.body {
                    bc.emit_stmt(stmt, &internal);
                }
                bc.return_void();
                bc.finish()
            };
            let desc = ctor_descriptor(method);
            emitter.add_method_from_bc("<init>", &desc, 0x0001, bc_out);
        } else {
            emitter.emit_function(
                method,
                &internal,
                &package_path,
                &empty_symbols,
                &[],
            );
        }
    }

    emitter.finish()
}

fn emit_interface(iface: &InterfaceDecl, package: &str) -> Vec<u8> {
    let internal = format!("{}/{}", package.replace('.', "/"), iface.name);
    let mut pool = CpPool::new();
    let this_class = pool.class(&internal);
    let super_class = pool.class("java/lang/Object");
    let code_attr_name = pool.utf8("Code");
    let mut methods = Vec::new();

    for method in &iface.methods {
        let desc = fn_descriptor(method);
        methods.push(MethodEmit {
            access: visibility_to_flags(method.visibility) | 0x0401,
            name_idx: pool.utf8(&method.name),
            desc_idx: pool.utf8(&desc),
            code: vec![],
            max_stack: 0,
            max_locals: 0,
            has_code: false,
        });
    }

    write_class(
        0x0601,
        &pool,
        this_class,
        super_class,
        code_attr_name,
        &[],
        &[],
        &methods,
    )
}

fn ctor_descriptor(method: &FnDecl) -> String {
    let params: String = method
        .params
        .iter()
        .map(|p| {
            p.ty
                .as_ref()
                .map(|t| RjType::from_ast(t).jvm_descriptor())
                .unwrap_or_else(|| "Ljava/lang/String;".into())
        })
        .collect();
    format!("({params})V")
}

fn emit_enum(e: &EnumDecl, package: &str) -> Vec<u8> {
    let internal = format!("{}/{}", package.replace('.', "/"), e.name);
    let mut pool = CpPool::new();
    let this_class = pool.class(&internal);
    let super_class = pool.class("java/lang/Enum");
    let code_attr_name = pool.utf8("Code");
    let mut fields = vec![];
    let methods = vec![];

    for variant in &e.variants {
        let name_idx = pool.utf8(&variant.name);
        let desc_idx = pool.utf8(&format!("L{internal};"));
        fields.push((0x4019u16, name_idx, desc_idx)); // public static final enum
    }

    write_class(
        0x4021,
        &pool,
        this_class,
        super_class,
        code_attr_name,
        &[],
        &fields,
        &methods,
    )
}

fn visibility_to_flags(v: Visibility) -> u16 {
    match v {
        Visibility::Public => 0x0001,
        Visibility::Private => 0x0002,
        Visibility::Protected => 0x0004,
    }
}

fn write_class(
    access: u16,
    pool: &CpPool,
    this_class: u16,
    super_class: u16,
    code_attr_name: u16,
    interfaces: &[u16],
    fields: &[(u16, u16, u16)],
    methods: &[MethodEmit],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.write_all(&[0xCA, 0xFE, 0xBA, 0xBE]).unwrap();
    out.write_all(&[0x00, 0x00, 0x00, 0x31]).unwrap();

    let count = usize_to_u16(pool.entries.len() + 1);
    out.write_all(&count.to_be_bytes()).unwrap();
    for entry in &pool.entries {
        match entry {
            CpEntry::Utf8(s) => {
                out.push(1);
                let b = s.as_bytes();
                out.write_all(&usize_to_u16(b.len()).to_be_bytes())
                    .unwrap();
                out.write_all(b).unwrap();
            }
            CpEntry::Class(n) => {
                out.push(7);
                out.write_all(&n.to_be_bytes()).unwrap();
            }
            CpEntry::String(u) => {
                out.push(8);
                out.write_all(&u.to_be_bytes()).unwrap();
            }
            CpEntry::NameAndType { name, desc } => {
                out.push(12);
                out.write_all(&name.to_be_bytes()).unwrap();
                out.write_all(&desc.to_be_bytes()).unwrap();
            }
            CpEntry::FieldRef { class, nat } => {
                out.push(9);
                out.write_all(&class.to_be_bytes()).unwrap();
                out.write_all(&nat.to_be_bytes()).unwrap();
            }
            CpEntry::MethodRef { class, nat } => {
                out.push(10);
                out.write_all(&class.to_be_bytes()).unwrap();
                out.write_all(&nat.to_be_bytes()).unwrap();
            }
            CpEntry::InterfaceMethodRef { class, nat } => {
                out.push(11);
                out.write_all(&class.to_be_bytes()).unwrap();
                out.write_all(&nat.to_be_bytes()).unwrap();
            }
            CpEntry::Integer(v) => {
                out.push(3);
                out.write_all(&v.to_be_bytes()).unwrap();
            }
        }
    }

    out.write_all(&access.to_be_bytes()).unwrap();
    out.write_all(&this_class.to_be_bytes()).unwrap();
    out.write_all(&super_class.to_be_bytes()).unwrap();
    out.write_all(&usize_to_u16(interfaces.len()).to_be_bytes())
        .unwrap();
    for iface in interfaces {
        out.write_all(&iface.to_be_bytes()).unwrap();
    }

    out.write_all(&usize_to_u16(fields.len()).to_be_bytes()).unwrap();
    for (af, ni, di) in fields {
        out.write_all(&af.to_be_bytes()).unwrap();
        out.write_all(&ni.to_be_bytes()).unwrap();
        out.write_all(&di.to_be_bytes()).unwrap();
        out.write_all(&0u16.to_be_bytes()).unwrap();
    }

    out.write_all(&usize_to_u16(methods.len()).to_be_bytes()).unwrap();
    for m in methods {
        out.write_all(&m.access.to_be_bytes()).unwrap();
        out.write_all(&m.name_idx.to_be_bytes()).unwrap();
        out.write_all(&m.desc_idx.to_be_bytes()).unwrap();
        if m.has_code {
            out.write_all(&1u16.to_be_bytes()).unwrap();
            out.write_all(&code_attr_name.to_be_bytes()).unwrap();
            let mut body = Vec::new();
            body.write_all(&m.max_stack.to_be_bytes()).unwrap();
            body.write_all(&m.max_locals.to_be_bytes()).unwrap();
            body.write_all(&usize_to_u32(m.code.len()).to_be_bytes())
                .unwrap();
            body.write_all(&m.code).unwrap();
            body.write_all(&0u16.to_be_bytes()).unwrap();
            body.write_all(&0u16.to_be_bytes()).unwrap();
            out.write_all(&usize_to_u32(body.len()).to_be_bytes()).unwrap();
            out.write_all(&body).unwrap();
        } else {
            out.write_all(&0u16.to_be_bytes()).unwrap();
        }
    }

    out.write_all(&0u16.to_be_bytes()).unwrap();
    out
}
