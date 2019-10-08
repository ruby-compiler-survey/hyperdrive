use trace::Trace;
use hyperdrive_ruby::ruby_special_consts_RUBY_Qnil;
use ir::OpCode::Snapshot;
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::ir::types::I64;
use cranelift::prelude::*;
use cranelift_module::*;
use cranelift_simplejit::*;
use cranelift_module::FuncOrDataId::Func;
use ir::*;
use ir::OpCode;
use vm;
use vm::*;

macro_rules! b1_2_value {
    ($x:ident, $builder:ident) => {{
        let fifth_bit = $builder.ins().bint(I64, $x);
        let fifth_bit = $builder.ins().ishl_imm(fifth_bit, 4);
        let third_bit = $builder.ins().bint(I64, $x);
        let third_bit = $builder.ins().ishl_imm(third_bit, 2);
        $builder.ins().iadd(fifth_bit, third_bit)
    }}
}

macro_rules! value_2_i64 {
    ($x:ident, $builder:ident) => {{
        $builder.ins().ushr_imm($x, 1)
    }}
}

macro_rules! i64_2_value {
    ($x:ident, $builder:ident) => {{
        let value = $builder.ins().ishl_imm($x, 1);
        $builder.ins().iadd_imm(value, 1)
    }}
}

pub struct Compiler<'a> {
    module: &'a mut Module<SimpleJITBackend>,
    builder: FunctionBuilder<'a>
}

impl <'a> Compiler<'a> {

    pub fn new(module: &'a mut Module<SimpleJITBackend>, builder: FunctionBuilder<'a>) -> Self {
        Self {
            module: module,
            builder: builder,
        }
    }

    pub fn compile(&mut self, trace: Trace){
        let entry_block = self.builder.create_ebb();
        let loop_block = self.builder.create_ebb();
        let original_loop_block = loop_block;
        self.builder.switch_to_block(entry_block);
        self.builder.append_ebb_params_for_function_params(entry_block);
        let ep = self.builder.ebb_params(entry_block)[0];
        let mut self_ = self.builder.ins().iconst(I64, trace.self_ as i64);
        self.builder.ins().jump(loop_block, &[]);
        self.builder.switch_to_block(loop_block);

        let mut ssa_values = vec![];
        for (i, node) in trace.nodes.iter().enumerate() {
            match &node.opcode {
                OpCode::Yarv(vm::OpCode::putself) => {
                    ssa_values.push(self_);
                },
                OpCode::Yarv(vm::OpCode::putnil) => {
                    ssa_values.push(self.builder.ins().iconst(I64, ruby_special_consts_RUBY_Qnil as i64));
                },
                OpCode::Yarv(vm::OpCode::putobject_INT2FIX_1_) => {
                    ssa_values.push(self.builder.ins().iconst(I64, 1 as i64));
                },
                OpCode::Yarv(vm::OpCode::putobject_INT2FIX_0_) => {
                    ssa_values.push(self.builder.ins().iconst(I64, 0 as i64));
                },
                OpCode::Yarv(vm::OpCode::opt_plus) => {
                    let a = ssa_values[node.ssa_operands[0]];
                    let b = ssa_values[node.ssa_operands[1]];
                    ssa_values.push(self.builder.ins().iadd(a, b));
                },
                OpCode::Yarv(vm::OpCode::getlocal_WC_0) => {
                    let offset = -8 * node.operands[0] as i32;
                    match node.type_ {
                        IrType::Yarv(ValueType::Fixnum) => {
                            let boxed = self.builder.ins().load(I64, MemFlags::new(), ep, offset);
                            let builder = &mut self.builder;
                            let unboxed = value_2_i64!(boxed, builder);
                            ssa_values.push(unboxed);
                        },
                        IrType::Yarv(_)=> {
                            let boxed = self.builder.ins().load(I64, MemFlags::new(), ep, offset);
                            ssa_values.push(boxed);
                        },
                        _ => panic!("unexpected: type {:?} in getlocal at offset: {} \n {:#?}", node.type_, i, trace.nodes),
                    }
                },
                OpCode::Yarv(vm::OpCode::setlocal_WC_0) => {
                    let offset = -8 * node.operands[0] as i32;
                    let ssa_ref = node.ssa_operands[0];

                    match trace.nodes[ssa_ref].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let unboxed = ssa_values[ssa_ref];
                            let builder = &mut self.builder;
                            let rvalue = i64_2_value!(unboxed, builder);
                            self.builder.ins().store(MemFlags::new(), rvalue, ep,  offset);
                        },
                        IrType::Yarv(_)|IrType::Internal(InternalType::Value) => {
                            let rvalue = ssa_values[ssa_ref];
                            self.builder.ins().store(MemFlags::new(), rvalue, ep,  offset);
                        },
                        IrType::Internal(InternalType::Bool) => {
                            let unboxed = ssa_values[ssa_ref];
                            let builder = &mut self.builder;
                            let rvalue = b1_2_value!(unboxed, builder);
                            self.builder.ins().store(MemFlags::new(), rvalue, ep,  offset);
                        }
                        _ => panic!("unexpect type {:?} in setlocal\n {:#?} ", trace.nodes[ssa_ref].type_, trace.nodes),
                    };
                    ssa_values.push(ssa_values[ssa_ref]);
                },
                OpCode::Yarv(vm::OpCode::putstring) => {
                    let unboxed = node.operands[0];
                    ssa_values.push(self.builder.ins().iconst(I64, unboxed as i64));
                },
                OpCode::Yarv(vm::OpCode::putobject) => {
                    let unboxed = node.operands[0] >> 1;
                    ssa_values.push(self.builder.ins().iconst(I64, unboxed as i64));
                },
                OpCode::Yarv(vm::OpCode::opt_lt) => {
                    let a = ssa_values[node.ssa_operands[0]];
                    let b = ssa_values[node.ssa_operands[1]];
                    let result = self.builder.ins().icmp(IntCC::SignedLessThan, a, b);
                    ssa_values.push(result);
                },
                OpCode::Yarv(vm::OpCode::opt_eq) => {
                    let a = ssa_values[node.ssa_operands[0]];
                    let b = ssa_values[node.ssa_operands[1]];
                    let result = self.builder.ins().icmp(IntCC::Equal, a, b);
                    ssa_values.push(result);
                },
                OpCode::Guard(IrType::Yarv(type_)) => {
                    let value = ssa_values[node.ssa_operands[0]];
                    let ssa_ref = node.ssa_operands[0];

                    match type_ {
                        ValueType::True => {
                            let loop_block = self.builder.create_ebb();
                            let side_exit_block = self.builder.create_ebb();
                            self.builder.ins().brz(value, side_exit_block, &[]);
                            self.builder.ins().jump(loop_block, &[]);
                            self.builder.switch_to_block(side_exit_block);
                            self.builder.ins().return_(&[]);
                            self.builder.switch_to_block(loop_block);
                        },
                        ValueType::False => {
                            let loop_block = self.builder.create_ebb();
                            let side_exit_block = self.builder.create_ebb();
                            self.builder.ins().brnz(value, side_exit_block, &[]);
                            self.builder.ins().jump(loop_block, &[]);
                            self.builder.switch_to_block(side_exit_block);
                            self.builder.ins().return_(&[]);
                            self.builder.switch_to_block(loop_block);
                        },
                        _ => panic!("unexpect type {:?} in guard\n {:#?} ", trace.nodes[ssa_ref].type_, trace.nodes),
                    };
                    ssa_values.push(ssa_values[ssa_ref]);
                },
                OpCode::Loop => { self.builder.ins().jump(original_loop_block, &[]); } ,
                OpCode::Yarv(vm::OpCode::duparray) => {
                    let array = node.operands[0];
                    let array = self.builder.ins().iconst(I64, array as i64);
                    if let Some(Func(id)) = self.module.get_name("_rb_ary_resurrect") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[array]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }

                },
                OpCode::Yarv(vm::OpCode::opt_send_without_block) => {
                    let call_cache = CallCache::new(node.operands[1] as *const _);
                    let func = call_cache.get_func() as *const u64;
                    let func = self.builder.ins().iconst(I64, func as i64);
                    let receiver = ssa_values[node.ssa_operands[0]];

                    let sig = Signature {
                        params: vec![AbiParam::new(I64)],
                        returns: vec![AbiParam::new(I64)],
                        call_conv: CallConv::SystemV,
                    };
                    let sig = self.builder.import_signature(sig);

                    let call = self.builder.ins().call_indirect(sig, func, &[receiver]);
                    let result = self.builder.inst_results(call)[0];
                    ssa_values.push(result);
                },
                OpCode::Yarv(vm::OpCode::opt_empty_p) => {
                    let receiver = ssa_values[node.ssa_operands[0]];

                    if let Some(Func(id)) = self.module.get_name("_rb_str_strlen") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[receiver]);
                        let count = self.builder.inst_results(call)[0];
                        let result = self.builder.ins().icmp_imm(IntCC::Equal, count, 0);
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::Yarv(vm::OpCode::pop) => { ssa_values.push(ssa_values[node.ssa_operands[0]]) },
                OpCode::Yarv(vm::OpCode::dup) => { ssa_values.push(ssa_values[node.ssa_operands[0]]) },
                OpCode::Yarv(vm::OpCode::opt_not) => {
                    let ssa_ref = node.ssa_operands[0];
                    let val = ssa_values[ssa_ref];

                    match trace.nodes[ssa_ref].type_ {
                        IrType::Internal(InternalType::Bool) => {
                            let result = self.builder.ins().bnot(val);
                            ssa_values.push(result);
                        },
                        IrType::Internal(InternalType::Value) => {
                            let result = self.builder.ins().icmp_imm(IntCC::Equal, val, 0);
                            ssa_values.push(result);
                        }
                        _ => panic!("unexpect type {:?} in opt_not\n {:#?} ", trace.nodes[ssa_ref].type_, trace.nodes),
                    };

                },
                OpCode::ArrayAppend => {
                    let array = ssa_values[node.ssa_operands[0]];
                    let unboxed_object = ssa_values[node.ssa_operands[1]];

                    let boxed_object = match trace.nodes[node.ssa_operands[1]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(unboxed_object, builder)
                        },
                        IrType::Yarv(ValueType::Fixnum)|IrType::Yarv(ValueType::RString) => {
                            unboxed_object
                        },
                        _ => panic!("unexpected type in ArrayAppend: {:#?} \n {:#?}", trace.nodes[node.ssa_operands[1]].type_, trace.nodes),
                    };

                    if let Some(Func(id)) = self.module.get_name("_rb_ary_push") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[array, boxed_object]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::NewArray => {
                    if let Some(Func(id)) = self.module.get_name("_rb_ary_new") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::ArrayGet => {
                    let array = ssa_values[node.ssa_operands[0]];
                    let maybe_unboxed = ssa_values[node.ssa_operands[1]];

                    let boxed_object = match trace.nodes[node.ssa_operands[1]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(maybe_unboxed, builder)
                        },
                        IrType::Yarv(ValueType::Fixnum) => {
                            maybe_unboxed
                        },
                        _ => panic!("unexpected type in ArrayReference: {:#?}", trace.nodes[node.ssa_operands[1]].type_),
                    };

                    if let Some(Func(id)) = self.module.get_name("_rb_ary_aref1") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[array, boxed_object]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::ArraySet => {
                    let array = ssa_values[node.ssa_operands[0]];
                    let key = ssa_values[node.ssa_operands[1]];
                    let value = ssa_values[node.ssa_operands[2]];

                    let boxed_key = match trace.nodes[node.ssa_operands[1]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(key, builder)
                        },
                        IrType::Yarv(_) => {
                            key
                        },
                        _ => panic!("unexpect type in ArraySet: {:#?}", trace.nodes[node.ssa_operands[1]].type_),
                    };

                    let boxed_value = match trace.nodes[node.ssa_operands[2]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(value, builder)
                        },
                        IrType::Yarv(_) => {
                            value
                        },
                        _ => panic!("unexpect type in ArrayAppend: {:#?} \n {:#?}", trace.nodes[node.ssa_operands[2]].type_, trace.nodes),
                    };

                    if let Some(Func(id)) = self.module.get_name("_rb_ary_store") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[array, boxed_key, boxed_value]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::NewHash => {
                    if let Some(Func(id)) = self.module.get_name("_rb_hash_new") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::HashSet => {
                    let hash = ssa_values[node.ssa_operands[0]];
                    let key = ssa_values[node.ssa_operands[1]];
                    let value = ssa_values[node.ssa_operands[2]];

                    let boxed_key = match trace.nodes[node.ssa_operands[1]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(key, builder)
                        },
                        IrType::Yarv(_) => {
                            key
                        },
                        _ => panic!("unexpect key type in HashSet: {:#?} \n {:#?}", trace.nodes[node.ssa_operands[1]].type_, trace.nodes),
                    };

                    let boxed_value = match trace.nodes[node.ssa_operands[2]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(value, builder)
                        },
                        IrType::Yarv(_) => {
                            value
                        },
                        _ => panic!("unexpect value type in HashSet: {:#?}", trace.nodes[node.ssa_operands[2]].type_),
                    };

                    if let Some(Func(id)) = self.module.get_name("_rb_hash_aset") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[hash, boxed_key, boxed_value]);
                        let _result = self.builder.inst_results(call)[0];
                        ssa_values.push(hash);
                    } else {
                        panic!("function not found!");
                    }
                },
                OpCode::HashGet => {
                    let hash = ssa_values[node.ssa_operands[0]];
                    let maybe_unboxed = ssa_values[node.ssa_operands[1]];

                    let boxed_object = match trace.nodes[node.ssa_operands[1]].type_ {
                        IrType::Internal(InternalType::I64) => {
                            let builder = &mut self.builder;
                            i64_2_value!(maybe_unboxed, builder)
                        },
                        IrType::Yarv(_) => {
                            maybe_unboxed
                        },
                        _ => panic!("unexpected type in HashGet: {:#?}", trace.nodes[node.ssa_operands[1]].type_),
                    };

                    if let Some(Func(id)) = self.module.get_name("_rb_hash_aref") {
                        let func_ref = self.module.declare_func_in_func(id, self.builder.func);
                        let call = self.builder.ins().call(func_ref, &[hash, boxed_object]);
                        let result = self.builder.inst_results(call)[0];
                        ssa_values.push(result);
                    } else {
                        panic!("function not found!");
                    }
                },
                Snapshot(_,new_self) => {
                    match new_self {
                        SsaOrValue::Ssa(reference) => { self_ = ssa_values[*reference] },
                        SsaOrValue::Value(_) => {},
                    }
                    ssa_values.push(self.builder.ins().iconst(I64, 0 as i64)) ;
                },
                _ => panic!("NYI: {:?}", node.opcode),
            };
        };
    }

    pub fn preview(&mut self) -> Result<String, String> {
        Ok(self.builder.display(None).to_string())
    }
}
