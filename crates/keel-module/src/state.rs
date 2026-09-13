use std::cell::RefCell;

use keel_core::AppState;
use keel_renderer::Renderer;
use keel_scheduler::FrameClock;

use crate::abi::KeelNativeStats;

pub(crate) struct ModuleState {
    pub(crate) app: AppState,
    pub(crate) renderer: Renderer,
    pub(crate) clock: FrameClock,
    pub(crate) stats: KeelNativeStats,
    pub(crate) completion_tenths_ms: u64,
    pub(crate) reserved_scroll_rows: u16,
}

impl ModuleState {
    pub(crate) fn new() -> Self {
        Self {
            app: AppState::default(),
            renderer: Renderer::new(),
            clock: FrameClock::new(),
            stats: KeelNativeStats::default(),
            completion_tenths_ms: 0,
            reserved_scroll_rows: 0,
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }
}

thread_local! {
    pub(crate) static STATE: RefCell<ModuleState> = RefCell::new(ModuleState::new());
}
