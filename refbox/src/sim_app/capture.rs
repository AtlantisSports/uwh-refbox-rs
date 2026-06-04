//! Dev-time maintenance command: render every front-display layout in the
//! (stable) simulator and save it as a PNG, so the Display Options page can show
//! a static preview without putting a live canvas on a settings page (which
//! crashes the Linux/tiny-skia renderer — see the design spec, Decision D).
//!
//! Invoked via `refbox --capture-previews <DIR>` (see `just capture-previews`).
//! It drives the real `SimRefBoxApp` rendering, so the pictures are faithful by
//! construction. Nothing here ships behaviour to end-user devices — it only
//! writes the committed PNGs that get embedded into the app.
//!
//! The `expect()` calls below are intentional dev-time-only panics (this binary
//! path never runs on a referee device); do not "fix" them into the app path.
//! The capture renders at scale factor 1.0 (pinned in `run_capture`) so the
//! committed PNGs are reproducible; the `just check-previews` drift guard assumes
//! a consistent render environment (CI runs at scale 1.0). If that ever proves
//! flaky across machines, switch `check-previews` to a tolerant pixel compare
//! (design spec Risk 2 / plan Task 5).

use std::path::{Path, PathBuf};

use iced::{
    Element, Length, Size, Subscription, Task,
    application::Appearance,
    widget::canvas::Canvas,
    window::{self, Screenshot},
};

use matrix_drawing::transmitted_data::{Brightness, TransmittedData};
use uwh_common::{bundles::BlackWhiteBundle, game_snapshot::GamePeriod};

use super::{Message as SimMessage, SimRefBoxApp, SimRefBoxAppFlags};
use crate::app::theme::{BLACK, WHITE};
use crate::sim_frame::{FrontDisplayLayout, SimFrame};

/// 16:9 capture resolution. Rendered larger than the on-screen preview so the
/// picture stays crisp when scaled in the UI.
const PREVIEW_W: u32 = 1280;
const PREVIEW_H: u32 = 720;

/// Number of rendered frames to wait after switching layouts before capturing,
/// so the new content is fully composited before the screenshot is taken.
/// 3 was confirmed sufficient during the capture spike (each variant captured
/// its own content, with no stale frames — verified by eye across all 10 PNGs).
const SETTLE_FRAMES: u8 = 3;

/// Sample game state shown in every preview: WHITE 5 – BLACK 3, First Half, 8:42.
pub(crate) fn sample_data(white_on_right: bool) -> TransmittedData {
    let snapshot = uwh_common::game_snapshot::GameSnapshotNoHeap {
        current_period: GamePeriod::FirstHalf,
        secs_in_period: 522, // 8:42
        scores: BlackWhiteBundle { black: 3, white: 5 },
        ..Default::default()
    };

    TransmittedData {
        white_on_right,
        flash: false,
        beep_test: false,
        brightness: Brightness::Low,
        snapshot,
    }
}

/// Every layout, in both starting-side orientations (10 pictures). Generating
/// both for every layout keeps the capture and the display logic uniform; the
/// few that don't differ by side just produce two identical pictures.
pub(crate) fn variants() -> Vec<(FrontDisplayLayout, bool)> {
    let layouts = [
        FrontDisplayLayout::Default,
        FrontDisplayLayout::Classic,
        FrontDisplayLayout::BigTime,
        FrontDisplayLayout::Corners,
        FrontDisplayLayout::ScoresOnly,
    ];
    let mut out = Vec::with_capacity(10);
    for layout in layouts {
        for white_on_right in [false, true] {
            out.push((layout, white_on_right));
        }
    }
    out
}

/// Stable filename (no extension) for a layout/side pair, e.g. `classic-white-right`.
pub(crate) fn file_stem(layout: FrontDisplayLayout, white_on_right: bool) -> String {
    let layout = match layout {
        FrontDisplayLayout::Default => "default",
        FrontDisplayLayout::Classic => "classic",
        FrontDisplayLayout::BigTime => "big-time",
        FrontDisplayLayout::Corners => "corners",
        FrontDisplayLayout::ScoresOnly => "scores-only",
    };
    let side = if white_on_right {
        "white-right"
    } else {
        "white-left"
    };
    format!("{layout}-{side}")
}

#[derive(Debug, Clone)]
enum Message {
    GotWindowId(Option<window::Id>),
    Frame,
    Captured(Screenshot),
}

struct CaptureApp {
    sim: SimRefBoxApp,
    variants: Vec<(FrontDisplayLayout, bool)>,
    index: usize,
    out_dir: PathBuf,
    window_id: Option<window::Id>,
    settle: u8,
    awaiting_shot: bool,
}

impl CaptureApp {
    fn new(out_dir: PathBuf) -> (Self, Task<Message>) {
        let variants = variants();
        // tcp_port 0 / no listener: we drive frames directly, never over TCP.
        let mut sim = SimRefBoxApp::new(SimRefBoxAppFlags {
            tcp_port: 0,
            sunlight_mode: false,
        })
        .0;
        push_variant(&mut sim, variants[0]);

        (
            Self {
                sim,
                variants,
                index: 0,
                out_dir,
                window_id: None,
                settle: SETTLE_FRAMES,
                awaiting_shot: false,
            },
            window::get_latest().map(Message::GotWindowId),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::GotWindowId(id) => {
                self.window_id = id;
                Task::none()
            }
            Message::Frame => {
                let Some(id) = self.window_id else {
                    return Task::none();
                };
                if self.awaiting_shot {
                    return Task::none();
                }
                if self.settle > 0 {
                    self.settle -= 1;
                    return Task::none();
                }
                self.awaiting_shot = true;
                window::screenshot(id).map(Message::Captured)
            }
            Message::Captured(shot) => {
                self.awaiting_shot = false;
                let (layout, white_on_right) = self.variants[self.index];
                save_png(&self.out_dir, layout, white_on_right, &shot);

                self.index += 1;
                if self.index >= self.variants.len() {
                    iced::exit()
                } else {
                    push_variant(&mut self.sim, self.variants[self.index]);
                    self.settle = SETTLE_FRAMES;
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        Canvas::new(&self.sim)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // `frames()` drives a continuous redraw, letting us count settle frames.
        window::frames().map(|_| Message::Frame)
    }

    fn style(&self, _theme: &iced::Theme) -> Appearance {
        Appearance {
            background_color: BLACK,
            text_color: WHITE,
        }
    }
}

fn push_variant(sim: &mut SimRefBoxApp, (layout, white_on_right): (FrontDisplayLayout, bool)) {
    // `update` returns Task::none() for NewSnapshot; nothing to schedule.
    let _ = sim.update(SimMessage::NewSnapshot(SimFrame {
        layout,
        data: sample_data(white_on_right),
    }));
}

fn save_png(dir: &Path, layout: FrontDisplayLayout, white_on_right: bool, shot: &Screenshot) {
    let buf = image::RgbaImage::from_raw(shot.size.width, shot.size.height, shot.bytes.to_vec())
        .expect("screenshot byte length matches its reported size");
    let path = dir.join(format!("{}.png", file_stem(layout, white_on_right)));
    buf.save(&path).expect("write preview png");
    println!("wrote {}", path.display());
}

/// Render every layout/side variant to a PNG in `out_dir`, then exit.
pub fn run_capture(out_dir: PathBuf) -> iced::Result {
    std::fs::create_dir_all(&out_dir).expect("create preview output directory");

    iced::application("Preview Capture", CaptureApp::update, CaptureApp::view)
        .subscription(CaptureApp::subscription)
        .style(CaptureApp::style)
        // Pin the UI scale to 1.0 so the capture doesn't pick up a non-1.0 app
        // scale; keeps the committed PNGs reproducible for the drift check.
        .scale_factor(|_| 1.0)
        .window(window::Settings {
            size: Size::new(PREVIEW_W as f32, PREVIEW_H as f32),
            resizable: false,
            ..Default::default()
        })
        // Only Roboto is needed: the sample data is digits + Latin team labels.
        // (The real display also loads CJK/Thai subsets; not needed for previews.)
        .font(include_bytes!("../../resources/Roboto-Medium.ttf").as_slice())
        .run_with(move || CaptureApp::new(out_dir.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_has_expected_score_and_clock() {
        let d = sample_data(false);
        assert_eq!(d.snapshot.scores.white, 5);
        assert_eq!(d.snapshot.scores.black, 3);
        assert_eq!(d.snapshot.secs_in_period, 522);
        assert_eq!(d.snapshot.current_period, GamePeriod::FirstHalf);
        assert!(!d.white_on_right);
        assert!(sample_data(true).white_on_right);
    }

    #[test]
    fn there_are_ten_variants_with_unique_filenames() {
        let v = variants();
        assert_eq!(v.len(), 10);
        let mut stems: Vec<String> = v.iter().map(|&(l, w)| file_stem(l, w)).collect();
        stems.sort();
        stems.dedup();
        assert_eq!(stems.len(), 10, "every variant must have a unique filename");
    }
}
