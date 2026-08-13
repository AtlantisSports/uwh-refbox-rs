use super::Mode;
use crate::config::GameSource;
use crate::portal_manager::PortalIndicatorState;
use uwh_common::{game_snapshot::GameSnapshot, uwhportal::schedule::TeamList};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ViewData<'a, 'b> {
    pub(super) snapshot: &'a GameSnapshot,
    pub(super) mode: Mode,
    /// Which game source is selected. The access-token page needs it to choose
    /// between the UWH Portal's menu directions and the generic wording a
    /// third-party site needs — refbox cannot know a custom site's admin
    /// screens, so the two cannot be merged into one string.
    pub(super) source: GameSource,
    pub(super) clock_running: bool,
    pub(super) teams: Option<&'b TeamList>,
    /// `Some` when a portal event is currently linked — the health tile
    /// renders on every banner-bearing page and the state is live. `None`
    /// when no event is linked (fresh install, or the operator has
    /// unlinked); the feature is dormant, the tile is not rendered, and
    /// the time banner falls back to the pre-feature layout.
    pub(super) portal_indicator: Option<PortalIndicatorState>,
    /// `true` when the refbox was launched with `--serial-port` (real LED
    /// panel connected). Used to gray out controls that only make sense
    /// without a real panel — currently just "Open New Display".
    pub(super) has_led_panel: bool,
    /// The custom site address as committed, which is the one the refbox
    /// actually talks to. The SITE row shows this rather than whatever is
    /// currently typed into the editor, so an address that has not been
    /// applied — or whose APPLY was refused — can never be mistaken for the
    /// address in use.
    pub(super) committed_site_url: &'a str,
}
