//! A text widget that shrinks its label until it fits the space available.
//!
//! Buttons here have fixed widths, so a translated label wider than its button
//! used to word-wrap badly: the first line broke mid-word and the second line
//! was pushed out of the button and never drawn at all. This wraps at word
//! gaps and, only if that is still not enough, shrinks the text — giving every
//! line of a label the same size and centring each line on its own.
//!
//! Measurement needs a `Renderer`, which iced only provides inside
//! `Widget::layout`. That is why this is a widget and not a plain helper. The
//! *decisions* are split out into `best_split` and `fit_layout`, which take
//! measurement as a callback so they can be tested without a window, a font,
//! or a renderer.
//!
//! Two iced 0.13 quirks shape the implementation:
//!
//! * `Wrapping::None` is accepted but never applied — `iced_graphics` defines
//!   `to_wrap` and never calls it, so the text engine always word-wraps. Every
//!   paragraph here is therefore laid out with unbounded width, which is the
//!   only reliable way to stop it wrapping behind our back.
//! * A paragraph's alignment only shifts the whole block at draw time; lines
//!   *within* one paragraph stay left-aligned. Each line therefore gets its own
//!   paragraph, which is what lets every line be centred individually.

use iced::{
    Element, Length, Rectangle, Size,
    alignment::{Horizontal, Vertical},
};
use iced_core::{
    Layout, Pixels, Point, Widget, layout, mouse, renderer,
    text::{self, IntoFragment, LineHeight, Paragraph, Shaping, Wrapping, paragraph::Plain},
    widget::{
        text::{Style as TextStyle, draw as draw_text},
        tree::{self, Tree},
    },
};
use std::borrow::Cow;

/// Smallest size a label may shrink to. Matches `theme::SMALL_TEXT`, the size
/// already used for the notes under language names, so it is known to be
/// legible on the refbox screen.
pub(super) const MIN_FIT_TEXT: f32 = 19.0;

/// Splits `line` in two at the break point that leaves the wider half as narrow
/// as possible. `None` when there is nowhere to break, i.e. a single long word,
/// which can only be shrunk.
///
/// A break may fall at a space, which is dropped, or after a `/`, which stays
/// with the first half so a value like `1/HALBZEIT` breaks as `1/` above
/// `HALBZEIT`. Breaking it lets both halves be drawn much larger than the one
/// long line would allow. Note `:` is deliberately not a break point — times
/// like `15:00` must stay whole.
fn best_split(measure: impl Fn(&str) -> f32, line: &str) -> Option<(String, String)> {
    let mut best: Option<(f32, &str, &str)> = None;

    for (index, character) in line.char_indices() {
        // Both are single-byte, so these slice boundaries are always valid.
        let (first, second) = match character {
            ' ' | '\t' => (&line[..index], &line[index + 1..]),
            '/' => (&line[..index + 1], &line[index + 1..]),
            _ => continue,
        };

        let (first, second) = (first.trim(), second.trim());
        if first.is_empty() || second.is_empty() {
            continue;
        }

        let widest = measure(first).max(measure(second));
        if best
            .as_ref()
            .is_none_or(|(narrowest, _, _)| widest < *narrowest)
        {
            best = Some((widest, first, second));
        }
    }

    best.map(|(_, first, second)| (first.to_string(), second.to_string()))
}

/// Picks how to lay a label out: which arrangement of lines, and at what size.
///
/// `line_sets` is ordered fewest-lines-first, and `candidates` largest-size
/// first. Size is the outer loop, so a label that needs two lines to stay at
/// full size gets two lines at full size rather than one shrunken line — which
/// is what an operator actually wants to read across a pool deck.
///
/// When nothing fits, the arrangement with the most lines is returned at the
/// smallest size: the least-bad way to show a label that is simply too long.
fn fit_layout(
    measure: impl Fn(&str, f32) -> f32,
    max_width: f32,
    candidates: &[f32],
    line_sets: &[Vec<String>],
) -> (usize, f32) {
    for &size in candidates {
        for (index, lines) in line_sets.iter().enumerate() {
            if lines.iter().all(|line| measure(line, size) <= max_width) {
                return (index, size);
            }
        }
    }

    (
        line_sets.len().saturating_sub(1),
        candidates.last().copied().unwrap_or(MIN_FIT_TEXT),
    )
}

/// Where a line of `line_width` starts inside a box of `box_width`.
///
/// Alignment is done here, by placing the line's box, rather than by telling the
/// text engine to align the paragraph — see the note in `layout` for why that
/// distinction matters to repainting.
fn line_left(align: Horizontal, box_width: f32, line_width: f32) -> f32 {
    match align {
        Horizontal::Left => 0.0,
        Horizontal::Center => (box_width - line_width) / 2.0,
        Horizontal::Right => box_width - line_width,
    }
}

/// Sizes to try, largest first: whole pixels from `max_size` down to the floor.
fn size_ladder(max_size: f32) -> Vec<f32> {
    let min = MIN_FIT_TEXT as i32;
    let max = (max_size.round() as i32).max(min);
    (min..=max).rev().map(|size| size as f32).collect()
}

/// A label drawn centred, wrapped at word gaps if needed, and shrunk only if
/// wrapping still leaves it too wide. Every line shares one size.
pub(super) struct FitText<'a> {
    lines: Vec<Cow<'a, str>>,
    /// `None` means "the app's default text size", matching what a plain
    /// `text()` widget would use.
    max_size: Option<f32>,
    /// Whether a single line may be re-wrapped at word gaps. Off when the caller
    /// supplied the line break itself, so its choice is never second-guessed.
    wrap: bool,
    width: Length,
    height: Length,
    align: Horizontal,
}

impl FitText<'_> {
    /// Sets the size the label starts at before any shrinking.
    pub(super) fn size(mut self, max_size: f32) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Sets how much vertical space to take. The default, `Fill`, centres the
    /// label in its parent — right inside a button. Use `Shrink` where the label
    /// is stacked above other content that needs the remaining room.
    pub(super) fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets how much horizontal space to take. The default, `Fill`, claims the
    /// whole width and centres within it. `Shrink` asks only for the width the
    /// text needs — iced lays such children out *before* the filling ones, so
    /// this is how a value gets first claim on the space and a label alongside
    /// it takes what is left.
    pub(super) fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Sets how each line sits within the width. Defaults to centred.
    pub(super) fn align_x(mut self, align: Horizontal) -> Self {
        self.align = align;
        self
    }
}

/// A label that shrinks to fit, wrapping at word gaps first.
///
/// A translation may carry its own line breaks — several `.ftl` entries are
/// written across two lines deliberately. Those win: the label is split on them
/// and never re-wrapped, so a translator's choice is not second-guessed.
pub(super) fn fit_text<'a>(line: impl IntoFragment<'a>) -> FitText<'a> {
    let fragment = line.into_fragment();
    let lines: Vec<Cow<'a, str>> = if fragment.contains('\n') {
        fragment
            .split('\n')
            .map(|line| Cow::Owned(line.trim().to_string()))
            .collect()
    } else {
        vec![fragment]
    };
    let wrap = lines.len() == 1;

    FitText {
        lines,
        max_size: None,
        wrap,
        width: Length::Fill,
        height: Length::Fill,
        align: Horizontal::Center,
    }
}

/// Two caller-chosen lines that shrink together, both at the same size. The
/// split is the caller's and is never re-wrapped.
pub(super) fn fit_text_lines<'a>(
    first: impl IntoFragment<'a>,
    second: impl IntoFragment<'a>,
) -> FitText<'a> {
    FitText {
        lines: vec![first.into_fragment(), second.into_fragment()],
        max_size: None,
        wrap: false,
        width: Length::Fill,
        height: Length::Fill,
        align: Horizontal::Center,
    }
}

/// What the widget remembers between frames. The app rebuilds its whole screen
/// on every clock tick, so re-running the search every frame would be wasteful
/// on the Pi.
struct State<P: Paragraph> {
    paragraphs: Vec<Plain<P>>,
    /// The chosen arrangement, after any wrapping.
    lines: Vec<String>,
    chosen: f32,
    key: Option<CacheKey>,
}

/// Everything the chosen layout depends on. An unchanged key means the previous
/// answer is still correct.
#[derive(PartialEq)]
struct CacheKey {
    lines: Vec<String>,
    available_width: f32,
    max_size: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for FitText<'_>
where
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph> {
            paragraphs: Vec::new(),
            lines: Vec::new(),
            chosen: MIN_FIT_TEXT,
            key: None,
        })
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let available = limits.max();
        let font = renderer.default_font();
        let max_size = self.max_size.unwrap_or_else(|| renderer.default_size().0);

        let key = CacheKey {
            lines: self.lines.iter().map(|line| line.to_string()).collect(),
            available_width: available.width,
            max_size,
        };

        if state.key.as_ref() != Some(&key) {
            // Unbounded width, so this is the true unwrapped width of the line.
            let measure = |line: &str, size: f32| -> f32 {
                Renderer::Paragraph::with_text(text::Text {
                    content: line,
                    bounds: Size::INFINITY,
                    size: Pixels(size),
                    line_height: LineHeight::default(),
                    font,
                    horizontal_alignment: self.align,
                    vertical_alignment: Vertical::Top,
                    shaping: Shaping::Basic,
                    wrapping: Wrapping::None,
                })
                .min_bounds()
                .width
            };

            let mut line_sets = vec![key.lines.clone()];
            if self.wrap && key.lines.len() == 1 {
                if let Some((first, second)) =
                    best_split(|line| measure(line, max_size), &key.lines[0])
                {
                    line_sets.push(vec![first, second]);
                }
            }

            let (index, size) =
                fit_layout(measure, available.width, &size_ladder(max_size), &line_sets);

            state.lines = line_sets.swap_remove(index);
            state.chosen = size;
            state.key = Some(key);
        }

        let size = state.chosen;
        let align = self.align;
        let line_height = LineHeight::default().to_absolute(Pixels(size)).0;

        let State {
            paragraphs, lines, ..
        } = state;
        paragraphs.resize_with(lines.len(), Plain::default);
        for (paragraph, line) in paragraphs.iter_mut().zip(lines.iter()) {
            paragraph.update(text::Text {
                content: line,
                bounds: Size::INFINITY,
                size: Pixels(size),
                line_height: LineHeight::default(),
                font,
                // Always anchored top-left, whatever `align` says — this widget
                // positions each line itself, below.
                //
                // iced's repaint tracking applies a paragraph's alignment offset
                // *after* clipping and using the clipped width
                // (`iced_graphics::text::visible_bounds`), while drawing applies
                // it *before* clipping using the full width. A centre-anchored
                // paragraph therefore reports a dirty rectangle half a text-width
                // away from where it draws, leaving half the text as stale
                // pixels; a right-anchored one is off by a full width. Anchoring
                // top-left makes that offset zero, so the two agree.
                horizontal_alignment: Horizontal::Left,
                vertical_alignment: Vertical::Top,
                shaping: Shaping::Basic,
                wrapping: Wrapping::None,
            });
        }

        let line_widths: Vec<f32> = paragraphs
            .iter()
            .map(|paragraph| paragraph.min_bounds().width)
            .collect();
        let block_height = line_height * lines.len() as f32;
        // Intrinsic width is the widest line, which is what `Shrink` asks for.
        let block_width = line_widths.iter().copied().fold(0.0f32, f32::max);
        let bounds = limits.resolve(
            self.width,
            self.height,
            Size::new(block_width, block_height),
        );
        // Centre the block of lines vertically, exactly as the `container`
        // wrappers used to. Two lines at the default size are a couple of pixels
        // taller than the inside of a button, so this offset can go slightly
        // negative — matching what the old layout did rather than shifting the
        // text down.
        let top = (bounds.height - block_height) / 2.0;

        let children = line_widths
            .iter()
            .enumerate()
            .map(|(index, &width)| {
                // Each line's box is exactly as wide as the line, placed where
                // the requested alignment wants it. Alignment is therefore a
                // property of the layout, never of the paragraph.
                let left = line_left(align, bounds.width, width);

                layout::Node::new(Size::new(width, line_height))
                    .move_to(Point::new(left, top + line_height * index as f32))
            })
            .collect();

        layout::Node::with_children(bounds, children)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        // Clip to our own bounds so a label still too wide at the floor is
        // trimmed rather than spilling onto the neighbouring button.
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };

        for (paragraph, line) in state.paragraphs.iter().zip(layout.children()) {
            draw_text(
                renderer,
                defaults,
                line,
                paragraph.raw(),
                TextStyle { color: None },
                &clip,
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<FitText<'a>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(widget: FitText<'a>) -> Self {
        Self::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ladder used by the full-width buttons: 29px down to 19px.
    fn candidates() -> Vec<f32> {
        size_ladder(29.0)
    }

    /// A fake ruler: every character is half the font size wide. Real
    /// measurement needs a renderer; the decision logic does not.
    fn ruler(line: &str, size: f32) -> f32 {
        line.chars().count() as f32 * size * 0.5
    }

    fn one(line: &str) -> Vec<Vec<String>> {
        vec![vec![line.to_string()]]
    }

    #[test]
    fn ladder_runs_from_the_starting_size_down_to_the_floor() {
        let ladder = size_ladder(29.0);
        assert_eq!(ladder.first(), Some(&29.0));
        assert_eq!(ladder.last(), Some(&MIN_FIT_TEXT));
        assert_eq!(ladder.len(), 11);
    }

    #[test]
    fn a_starting_size_below_the_floor_still_yields_one_candidate() {
        assert_eq!(size_ladder(10.0), vec![MIN_FIT_TEXT]);
    }

    #[test]
    fn keeps_the_largest_size_when_the_label_already_fits() {
        // 4 characters at 29px is 58 wide, comfortably inside 100.
        assert_eq!(
            fit_layout(ruler, 100.0, &candidates(), &one("ABCD")),
            (0, 29.0)
        );
    }

    #[test]
    fn steps_down_to_the_first_size_that_fits() {
        // 8 characters, so width is 4 * size: it fits once size <= 25.
        // 25 is distinct from both 29 (the maximum) and 19 (the floor), so a
        // pass here cannot be confused with "fell back to a default".
        assert_eq!(
            fit_layout(ruler, 100.0, &candidates(), &one("ABCDEFGH")),
            (0, 25.0)
        );
    }

    #[test]
    fn a_label_exactly_as_wide_as_the_space_counts_as_fitting() {
        // 8 characters at 26px is 104 exactly; at 27px it is 108 and does not.
        assert_eq!(
            fit_layout(ruler, 104.0, &candidates(), &one("ABCDEFGH")),
            (0, 26.0)
        );
    }

    #[test]
    fn returns_the_floor_when_nothing_fits() {
        assert_eq!(
            fit_layout(ruler, 10.0, &candidates(), &one("ABCDEFGH")),
            (0, MIN_FIT_TEXT)
        );
    }

    #[test]
    fn an_empty_label_keeps_the_full_size() {
        assert_eq!(fit_layout(ruler, 100.0, &candidates(), &one("")), (0, 29.0));
    }

    #[test]
    fn no_room_at_all_falls_back_to_the_floor() {
        // Happens transiently while a window is being laid out; it corrects
        // itself on the next pass.
        assert_eq!(
            fit_layout(ruler, 0.0, &candidates(), &one("AB")),
            (0, MIN_FIT_TEXT)
        );
    }

    #[test]
    fn the_wider_line_governs_the_size_of_both_lines() {
        // The short line alone would allow the full 29px; the long one caps it
        // at 25, and both lines share that one size.
        let lines = vec![vec!["AB".to_string(), "ABCDEFGH".to_string()]];
        assert_eq!(fit_layout(ruler, 100.0, &candidates(), &lines), (0, 25.0));
    }

    #[test]
    fn prefers_one_line_when_it_fits_at_full_size() {
        let sets = vec![
            vec!["AB CD".to_string()],
            vec!["AB".to_string(), "CD".to_string()],
        ];
        assert_eq!(fit_layout(ruler, 200.0, &candidates(), &sets), (0, 29.0));
    }

    #[test]
    fn prefers_two_lines_at_a_bigger_size_over_one_shrunken_line() {
        // "JETZT STARTEN" is 13 characters and never fits on one line here.
        // Split, the longer half is 7 characters, which fits at 28px.
        // A one-line answer would have had to shrink far further.
        let sets = vec![
            vec!["JETZT STARTEN".to_string()],
            vec!["JETZT".to_string(), "STARTEN".to_string()],
        ];
        assert_eq!(fit_layout(ruler, 100.0, &candidates(), &sets), (1, 28.0));
    }

    #[test]
    fn splitting_balances_the_two_halves() {
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(
            best_split(measure, "ZUM TESTEN HALTEN"),
            Some(("ZUM TESTEN".to_string(), "HALTEN".to_string()))
        );
    }

    #[test]
    fn lines_of_different_widths_are_each_centred_on_their_own() {
        // The whole point of giving every line its own box: a short line and a
        // long one in the same label must not share a left edge, which is what
        // made German TEAM WARNING look left-aligned.
        assert_eq!(line_left(Horizontal::Center, 100.0, 40.0), 30.0);
        assert_eq!(line_left(Horizontal::Center, 100.0, 80.0), 10.0);
    }

    #[test]
    fn left_and_right_alignment_pin_the_matching_edge() {
        assert_eq!(line_left(Horizontal::Left, 100.0, 40.0), 0.0);
        assert_eq!(line_left(Horizontal::Right, 100.0, 40.0), 60.0);
    }

    #[test]
    fn splitting_breaks_after_a_slash_and_keeps_it() {
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(
            best_split(measure, "1/HALBZEIT"),
            Some(("1/".to_string(), "HALBZEIT".to_string()))
        );
    }

    #[test]
    fn a_time_is_never_split_at_its_colon() {
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(best_split(measure, "15:00"), None);
    }

    #[test]
    fn splitting_a_single_word_is_not_possible() {
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(best_split(measure, "VERWARNUNG"), None);
    }
}
