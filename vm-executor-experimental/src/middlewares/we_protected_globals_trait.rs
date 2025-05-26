use std::fmt::Debug;

use super::{Breakpoints, Metering, OpcodeControl};

pub trait MiddlewareWithProtectedGlobals: Debug {
    fn protected_globals(&self) -> Vec<u32>;
}

impl MiddlewareWithProtectedGlobals for OpcodeControl {
    fn protected_globals(&self) -> Vec<u32> {
        vec![
            self.get_memory_grow_count_global_index().as_u32(),
            self.get_operand_backup_global_index().as_u32(),
        ]
    }
}

impl MiddlewareWithProtectedGlobals for Breakpoints {
    fn protected_globals(&self) -> Vec<u32> {
        vec![self.get_breakpoint_value_global_index().as_u32()]
    }
}

impl MiddlewareWithProtectedGlobals for Metering {
    fn protected_globals(&self) -> Vec<u32> {
        vec![
            self.get_points_limit_global_index().as_u32(),
            self.get_points_used_global_index().as_u32(),
        ]
    }
}
