use super::*;

impl Recorder {
    pub fn record_comparison(&mut self, _thread: Thread, instruction: Instruction) {
        let b = self.stack_pop();
        let a = self.stack_pop();
        self.nodes.push(IrNode {
            type_: IrType::Internal(InternalType::Bool),
            opcode: ir::OpCode::Yarv(instruction.opcode()),
            operands: vec![],
            ssa_operands: vec![a, b],
        });
        self.stack_push(self.nodes.len() - 1);
    }
}
