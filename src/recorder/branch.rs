use super::*;

impl Recorder {
    pub fn record_branch(&mut self, thread: Thread, _instruction: Instruction) {
        let value: Value = unsafe { *thread.get_sp().offset(-1) }.into();
        let popped = self.stack_pop();
        self.nodes.push(IrNode {
            type_: IrType::None,
            opcode: ir::OpCode::Guard(IrType::Yarv(value.type_())),
            operands: vec![],
            ssa_operands: vec![popped],
        });
        self.nodes.push(IrNode {
            type_: IrType::None,
            opcode: ir::OpCode::Snapshot(
                thread.get_pc() as u64 + 8,
                SsaOrValue::Value(thread.get_self()),
            ),
            operands: vec![],
            ssa_operands: vec![],
        });
    }
}
