mod branch;
mod dup;
mod duparray;
mod getlocal_wc_0;
mod opt_aref;
mod opt_eq;
mod opt_empty_p;
mod opt_lt;
mod opt_ltlt;
mod opt_not;
mod opt_plus;
mod opt_send_without_block;
mod pop;
mod putnil;
mod putobject;
mod putobject_int2fix_1_;
mod putstring;
mod setlocal_wc_0;

use hyperdrive_ruby::rb_method_type_t_VM_METHOD_TYPE_CFUNC;
use ir;
use ir::*;
use trace::IrNodes;
use vm::*;
use vm::OpCode;

#[derive(Clone, Debug)]
pub struct Recorder {
    pub nodes: IrNodes,
    stack: Vec<SsaRef>,
    pub anchor: u64,
    pub complete: bool,
}

impl Recorder {
    pub fn new(anchor: u64) -> Self {
        Self {
            nodes: vec![],
            stack: vec![],
            anchor: anchor,
            complete: false,
        }
    }

    pub fn record_instruction(&mut self, thread: Thread) {
        let instruction = Instruction::new(thread.get_pc());
        let opcode = instruction.opcode();

        if !self.nodes.is_empty() && thread.get_pc() as u64 == self.anchor {
            self.nodes.push(
                IrNode {
                    type_: IrType::None,
                    opcode: ir::OpCode::Loop,
                    operands: vec![],
                    ssa_operands: vec![],
                }
            );
            self.complete = true;
            return
        }

        match opcode {
            OpCode::branchif|OpCode::branchunless => { branch::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::dup => { dup::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::duparray => { duparray::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::getlocal_WC_0 => { getlocal_wc_0::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_aref => { opt_aref::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_eq => { opt_eq::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_empty_p => { opt_empty_p::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_lt => { opt_lt::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_ltlt => { opt_ltlt::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_not => { opt_not::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_plus => { opt_plus::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::opt_send_without_block => { opt_send_without_block::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::pop => { pop::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::putobject => { putobject::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::putobject_INT2FIX_1_ => { putobject_int2fix_1_::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::putnil => { putnil::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::putstring => { putstring::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::setlocal_WC_0 => { setlocal_wc_0::record(&mut self.nodes, &mut self.stack, instruction, thread) },
            OpCode::putself|OpCode::leave => {},
            _ => panic!("Recording NYI: {:?}", opcode),
        }
    }
}