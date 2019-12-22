use super::*;

impl Recorder {
    pub fn record_opt_size(&mut self, thread: Thread, instruction: Instruction) {
        let popped = self.stack_pop();

        self.nodes.push(IrNode::Basic {
            type_: IrType::Internal(InternalType::I64),
            opcode: ir::OpCode::Yarv(instruction.opcode()),
            operands: vec![],
            ssa_operands: vec![popped],
        });
        self.stack_push(self.nodes.len() - 1);
    }
}