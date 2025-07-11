use crate::bytecompiler::{
    ByteCompiler,
    jump_control::{JumpRecord, JumpRecordAction, JumpRecordKind},
};
use boa_ast::statement::Break;

impl ByteCompiler<'_> {
    /// Compile a [`Break`] `boa_ast` node
    pub(crate) fn compile_break(&mut self, node: Break, _use_expr: bool) {
        let actions = self.break_jump_record_actions(node);

        JumpRecord::new(JumpRecordKind::Break, actions).perform_actions(Self::DUMMY_ADDRESS, self);
    }

    fn break_jump_record_actions(&self, node: Break) -> Vec<JumpRecordAction> {
        let mut actions = Vec::default();
        for (i, info) in self.jump_info.iter().enumerate().rev() {
            let count = self.jump_info_open_environment_count(i);
            actions.push(JumpRecordAction::PopEnvironments { count });

            if !info.in_finally()
                && let Some(finally_throw) = info.finally_throw
            {
                actions.push(JumpRecordAction::HandleFinally {
                    index: info.jumps.len() as u32,
                    finally_throw,
                });
                actions.push(JumpRecordAction::Transfer { index: i as u32 });
            }

            if let Some(label) = node.label() {
                if info.label() == Some(label) {
                    actions.push(JumpRecordAction::Transfer { index: i as u32 });
                    break;
                }
            } else if info.is_loop() || info.is_switch() {
                actions.push(JumpRecordAction::Transfer { index: i as u32 });
                break;
            }
        }

        actions.reverse();
        actions
    }
}
