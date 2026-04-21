use super::Mode;
use crate::portal_manager::PortalIndicatorState;
use uwh_common::{game_snapshot::GameSnapshot, uwhportal::schedule::TeamList};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ViewData<'a, 'b> {
    pub(super) snapshot: &'a GameSnapshot,
    pub(super) mode: Mode,
    pub(super) clock_running: bool,
    pub(super) teams: Option<&'b TeamList>,
    pub(super) portal_indicator: PortalIndicatorState,
}
