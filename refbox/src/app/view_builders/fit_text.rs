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

/// Whether a line may end after this character.
///
/// True for the CJK scripts, where breaking between characters is how wrapping
/// normally works. Kana, ideographs, Hangul syllables and full-width forms all
/// qualify; anything else (Latin, Thai, digits, punctuation) does not, so those
/// still break only at spaces.
fn breaks_after(character: char) -> bool {
    matches!(
        character as u32,
        0x3040..=0x30FF     // hiragana and katakana
        | 0x3400..=0x4DBF   // CJK extension A
        | 0x4E00..=0x9FFF   // CJK unified ideographs
        | 0xF900..=0xFAFF   // CJK compatibility ideographs
        | 0xAC00..=0xD7AF   // Hangul syllables
        | 0xFF00..=0xFF60 // full-width forms
    )
}

/// Whether a line may begin with this character.
///
/// An abridged form of the Japanese kinsoku rules: a line must not start with
/// closing punctuation or a long-vowel mark, which would read as a typo.
fn breaks_before(character: char) -> bool {
    breaks_after(character)
        && !matches!(
            character,
            '、' | '。'
                | '，'
                | '．'
                | '）'
                | '」'
                | '』'
                | '】'
                | '〕'
                | '！'
                | '？'
                | 'ー'
                | '・'
                | 'ｰ'
        )
}

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
    let mut previous: Option<char> = None;

    for (index, character) in line.char_indices() {
        // ' ', '\t' and '/' are single-byte, so those boundaries are valid; the
        // CJK case slices at `index`, which is a char boundary by construction.
        let (first, second) = match character {
            ' ' | '\t' => (&line[..index], &line[index + 1..]),
            '/' => (&line[..index + 1], &line[index + 1..]),
            // Between two CJK characters, which is how those scripts are broken:
            // they have no spaces, so without this a long Japanese or Chinese
            // label has nowhere to wrap and can only shrink -- to the floor and
            // then to clipping.
            _ if previous.is_some_and(breaks_after) && breaks_before(character) => {
                (&line[..index], &line[index..])
            }
            _ => {
                previous = Some(character);
                continue;
            }
        };
        previous = Some(character);

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

/// Index of the first (largest) candidate at which **every** string in `shared`
/// fits `max_width`.
///
/// This is how separate widgets agree on one size. The game-time banner draws the
/// period label and the timeout label in different columns, but they must look
/// like a pair, so each is fitted against both strings and the longer one governs.
/// Returns the last index when nothing fits, so the caller still gets a ladder.
fn shared_start(
    measure: impl Fn(&str, f32) -> f32,
    max_width: f32,
    candidates: &[f32],
    shared: &[String],
    wrap: bool,
) -> usize {
    // A shared string "fits" on the same terms the drawn one does: on one line,
    // or -- when wrapping is allowed -- split in two with its wider half fitting.
    // Judging shared strings as single lines only would force everything down to
    // the size the longest one needs unsplit, which is not what it will be drawn at.
    let fits = |line: &str, size: f32| {
        if measure(line, size) <= max_width {
            return true;
        }
        wrap && best_split(|part| measure(part, size), line).is_some_and(|(first, second)| {
            measure(&first, size).max(measure(&second, size)) <= max_width
        })
    };

    candidates
        .iter()
        .position(|&size| shared.iter().all(|line| fits(line, size)))
        .unwrap_or(candidates.len().saturating_sub(1))
}

/// Width of the widest string this label must be able to hold at `size`, or
/// `0.0` where none were named.
///
/// This is what a `Shrink` box claims over and above the text it is drawing. A
/// box that asks only for its current text moves every time that text changes,
/// and drags whatever shares the row with it: the keypad's entered value would
/// take width back from its own label one digit at a time, and the label would
/// re-fit itself on every keystroke. Claiming the widest value the label may
/// ever show keeps both boxes still.
///
/// Each string is measured as it would be drawn in `max_width`, which is what
/// `fit_layout` decides too: one line where one line fits, otherwise split with
/// its wider half governing. Reserving the split width of a string that will be
/// drawn whole under-reserves, and the box grows the moment the text becomes
/// that value; reserving the whole width of one that will be drawn wrapped
/// claims room it never occupies, and can leave a filling sibling nothing.
fn shared_width(
    measure: impl Fn(&str, f32) -> f32,
    size: f32,
    shared: &[String],
    wrap: bool,
    max_width: f32,
) -> f32 {
    let drawn = |line: &str| -> f32 {
        let whole = measure(line, size);
        if !wrap || whole <= max_width {
            return whole;
        }
        best_split(|part| measure(part, size), line)
            .map(|(first, second)| measure(&first, size).max(measure(&second, size)))
            .unwrap_or(whole)
    };

    shared.iter().map(|line| drawn(line)).fold(0.0f32, f32::max)
}

/// Sizes to try, largest first: whole pixels from `max_size` down to `min_size`.
fn size_ladder(max_size: f32, min_size: f32) -> Vec<f32> {
    let min = min_size.round().max(1.0) as i32;
    let max = (max_size.round() as i32).max(min);
    (min..=max).rev().map(|size| size as f32).collect()
}

/// A label drawn centred, wrapped at word gaps if needed, and shrunk only if
/// wrapping still leaves it too wide. Every line shares one size.
pub(super) struct FitText<'a, Theme> {
    lines: Vec<Cow<'a, str>>,
    /// `None` means "the app's default text size", matching what a plain
    /// `text()` widget would use.
    max_size: Option<f32>,
    /// Smallest size to shrink to. Defaults to `MIN_FIT_TEXT`, which suits
    /// buttons; the game-time banner sets its own, lower, floor because losing
    /// the clock is far worse than a small period label.
    min_size: f32,
    /// Whether a single line may be re-wrapped at word gaps. Off when the caller
    /// supplied the line break itself, or where a second line would push a
    /// neighbouring readout out of its box.
    wrap: bool,
    width: Length,
    height: Length,
    align: Horizontal,
    /// Optional state-dependent colour. `None` inherits, as a plain `text()` does.
    style: Option<fn(&Theme) -> TextStyle>,
    /// Strings that are measured but not drawn, so that several widgets showing
    /// related values settle on one size instead of each fitting alone.
    shared: Vec<String>,
}

impl<Theme> FitText<'_, Theme> {
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

    /// Shrinks rather than wrapping at word gaps.
    ///
    /// The default suits buttons, which have room for a second line. It is wrong
    /// in the game-time banner, where the period label sits above the clock in a
    /// box with no spare height: a second line there pushes the clock out of the
    /// banner and it is not drawn at all.
    pub(super) fn no_wrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    /// Sets the smallest size to shrink to, overriding `MIN_FIT_TEXT`.
    pub(super) fn min_size(mut self, min_size: f32) -> Self {
        self.min_size = min_size;
        self
    }

    /// Sets a state-dependent colour, as the game-time banner needs: the period
    /// and clock are green, yellow or red depending on the state of play.
    pub(super) fn style(mut self, style: fn(&Theme) -> TextStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sizes this label so that every string in `shared` would also fit, then
    /// draws only its own text.
    ///
    /// Use it to keep related readouts consistent: the game-time banner's period
    /// label and timeout label live in separate columns of equal width, and
    /// without this the short one renders at full size next to a shrunken long
    /// one, so the two stop looking like a pair. The longest string governs.
    ///
    /// A `Shrink` box additionally claims the width of the widest of them, so it
    /// does not resize as its own text changes — see `shared_width`. For a
    /// `Fill` or `Fixed` box, which cannot use an intrinsic width, this is size
    /// agreement and nothing more.
    pub(super) fn shared_with(mut self, shared: Vec<String>) -> Self {
        self.shared = shared;
        self
    }
}

/// A label that shrinks to fit, wrapping at word gaps first.
///
/// A translation may carry its own line breaks — several `.ftl` entries are
/// written across two lines deliberately. Those win: the label is split on them
/// and never re-wrapped, so a translator's choice is not second-guessed.
pub(super) fn fit_text<'a, Theme>(line: impl IntoFragment<'a>) -> FitText<'a, Theme> {
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
        min_size: MIN_FIT_TEXT,
        wrap,
        width: Length::Fill,
        height: Length::Fill,
        align: Horizontal::Center,
        style: None,
        shared: Vec::new(),
    }
}

/// Two caller-chosen lines that shrink together, both at the same size. The
/// split is the caller's and is never re-wrapped.
pub(super) fn fit_text_lines<'a, Theme>(
    first: impl IntoFragment<'a>,
    second: impl IntoFragment<'a>,
) -> FitText<'a, Theme> {
    FitText {
        lines: vec![first.into_fragment(), second.into_fragment()],
        max_size: None,
        min_size: MIN_FIT_TEXT,
        wrap: false,
        width: Length::Fill,
        height: Length::Fill,
        align: Horizontal::Center,
        style: None,
        shared: Vec::new(),
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
    /// Width of the widest shared string at the largest size they all fit,
    /// which a `Shrink` box claims even when the text being drawn is narrower.
    /// Deliberately *not* measured at `chosen`: a value that outgrew its
    /// reservation is drawn smaller than the reservation was taken at, and the
    /// box must not follow it. `0.0` where no shared strings were named.
    reserved: f32,
    key: Option<CacheKey>,
}

/// Everything the chosen layout depends on. An unchanged key means the previous
/// answer is still correct.
#[derive(PartialEq)]
struct CacheKey {
    lines: Vec<String>,
    shared: Vec<String>,
    available_width: f32,
    max_size: f32,
    /// Whether the shared strings are being reserved room, which only a
    /// `Shrink` box can use. Part of the key so a caller that changes its width
    /// re-measures instead of keeping an answer taken under the other rule.
    reserve: bool,
    /// Whether a line may be re-wrapped, which decides both the arrangement and
    /// how much room the shared strings claim. `shared_elements` flips this
    /// between frames, so an unkeyed answer could outlive the rule it was taken
    /// under.
    wrap: bool,
    /// The ladder's floor. It sets where the search stops, and so also the size
    /// `reserved` is measured at once nothing larger fits. Keyed for the same
    /// reason as `wrap`.
    min_size: f32,
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for FitText<'_, Theme>
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
            reserved: 0.0,
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
            shared: self.shared.clone(),
            available_width: available.width,
            max_size,
            // `limits.resolve` ignores the intrinsic width of a `Fill` or
            // `Fixed` box, so reserving room for a string it is not drawing
            // would be measured and then thrown away -- wasted work on every
            // cache miss, and the banner's clock misses once a second on the
            // Pi. It also keeps `shared_with` meaning only what it has always
            // meant for those callers: agree on a size.
            reserve: self.width == Length::Shrink,
            wrap: self.wrap,
            min_size: self.min_size,
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

            // Begin the ladder at the largest size where every shared string
            // fits, so related readouts settle on one size rather than each
            // fitting alone.
            let ladder = size_ladder(max_size, self.min_size);
            let start = shared_start(measure, available.width, &ladder, &key.shared, self.wrap);

            // Room to hold every value this label may be asked to show, taken
            // at the size they all fit rather than at the size finally chosen,
            // so it does not move when the drawn text does.
            let reserving = key.reserve && !key.shared.is_empty();
            let reserved = if reserving {
                // Clamped here rather than left to `limits.resolve`: a claim
                // wider than the space would otherwise become the ceiling the
                // text is fitted to, and the text would be sized for a box it
                // is never granted -- clipped instead of shrunk.
                shared_width(
                    measure,
                    ladder[start],
                    &key.shared,
                    key.wrap,
                    available.width,
                )
                .min(available.width)
            } else {
                0.0
            };

            // A reserving box is `reserved` wide whatever it holds, so the text
            // is fitted to that and not to the whole space. A value that
            // outgrows what its caller declared then shrinks, rather than
            // widening the box and taking the width from whatever shares the
            // row: the GameNumber keypad reserves four digits, `next_game_number`
            // increments without a cap, and a label robbed of that width can
            // shrink past its floor and clip.
            //
            // Note what this trades. Inside the range a caller declares, its
            // text never changes size -- the whole point. Outside it, the text
            // shrinks on the keystroke that crosses the boundary, which on the
            // GameNumber page means a manual 9999 rolling to "10000" is drawn
            // smaller than "1000" was. That is a worse jump than the old rigid
            // split gave (five digits fitted its 113px share), and it is still
            // the better trade: a shrunken number is legible, a clipped label is
            // not, and the fix for the real defect is a cap that matches what
            // the app can generate.
            let fit_width = if reserving { reserved } else { available.width };
            let (index, size) = fit_layout(measure, fit_width, &ladder[start..], &line_sets);

            state.lines = line_sets.swap_remove(index);
            state.chosen = size;
            state.reserved = reserved;
            state.key = Some(key);
        }

        let size = state.chosen;
        let reserved = state.reserved;
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
        // Intrinsic width is the widest line, which is what `Shrink` asks for --
        // or the widest value this label must be able to hold, where it named
        // some, so that the box stays put as the value changes.
        let block_width = line_widths
            .iter()
            .copied()
            .fold(0.0f32, f32::max)
            .max(reserved);
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
        theme: &Theme,
        defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        // No style set means inherit, which is what a plain `text()` inside a
        // button does and what keeps every button's own colours working.
        let appearance = match self.style {
            Some(style) => style(theme),
            None => TextStyle { color: None },
        };

        // Clip to our own bounds so a label still too wide at the floor is
        // trimmed rather than spilling onto the neighbouring button.
        let Some(clip) = layout.bounds().intersection(viewport) else {
            return;
        };

        for (paragraph, line) in state.paragraphs.iter().zip(layout.children()) {
            draw_text(renderer, defaults, line, paragraph.raw(), appearance, &clip);
        }
    }
}

impl<'a, Message, Theme, Renderer> From<FitText<'a, Theme>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(widget: FitText<'a, Theme>) -> Self {
        Self::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ladder used by the full-width buttons: 29px down to 19px.
    fn candidates() -> Vec<f32> {
        size_ladder(29.0, MIN_FIT_TEXT)
    }

    /// A fake ruler: every character is half the font size wide. Real
    /// measurement needs a renderer; the decision logic does not.
    fn ruler(line: &str, size: f32) -> f32 {
        line.chars().count() as f32 * size * 0.5
    }

    /// Digits at Roboto Medium's real advance, 1164/2048 em, so a test about the
    /// keypad's own geometry asserts on the true margins rather than on an
    /// artefact of `ruler`'s half-em guess. Every one of the three bundled faces
    /// gives all ten digits a single advance, but not the same one: the CJK
    /// subset's is 5% wider (614/1024 em), so Roboto is the tighter case for
    /// "does the old share overflow" and the smaller reservation for the rest.
    fn digit_ruler(line: &str, size: f32) -> f32 {
        line.chars().count() as f32 * size * (1164.0 / 2048.0)
    }

    fn one(line: &str) -> Vec<Vec<String>> {
        vec![vec![line.to_string()]]
    }

    #[test]
    fn ladder_runs_from_the_starting_size_down_to_the_floor() {
        let ladder = size_ladder(29.0, MIN_FIT_TEXT);
        assert_eq!(ladder.first(), Some(&29.0));
        assert_eq!(ladder.last(), Some(&MIN_FIT_TEXT));
        assert_eq!(ladder.len(), 11);
    }

    #[test]
    fn a_starting_size_below_the_floor_still_yields_one_candidate() {
        assert_eq!(size_ladder(10.0, MIN_FIT_TEXT), vec![MIN_FIT_TEXT]);
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
    fn a_lower_floor_lets_a_label_shrink_further_rather_than_stop() {
        // The game-time banner needs this: on one line, "ERSTE HALBZEIT" (14
        // characters) needs 7 * size, so 100px of width demands ~14px — below the
        // 19px floor that suits buttons. With the button floor it would stop at 19
        // and overflow; with a lower floor it shrinks and fits.
        let sets = vec![vec!["ERSTE HALBZEIT".to_string()]];
        assert_eq!(
            fit_layout(ruler, 100.0, &size_ladder(29.0, MIN_FIT_TEXT), &sets),
            (0, MIN_FIT_TEXT)
        );
        assert_eq!(
            fit_layout(ruler, 100.0, &size_ladder(29.0, 12.0), &sets),
            (0, 14.0)
        );
    }

    #[test]
    fn a_time_is_never_split_at_its_colon() {
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(best_split(measure, "15:00"), None);
    }

    #[test]
    fn japanese_splits_between_characters_having_no_spaces() {
        // The alarm hint. Thirteen full-width glyphs with nowhere to wrap could
        // only shrink, and at its starting size it was wider than its button, so
        // it clipped at both ends. Breaking between characters is how these
        // scripts wrap.
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(
            best_split(measure, "またはスペースキーを長押し"),
            Some(("またはスペー".to_string(), "スキーを長押し".to_string()))
        );
    }

    #[test]
    fn a_line_never_begins_with_closing_punctuation() {
        // Abridged kinsoku: breaking before 。 would strand it at the start of a
        // line, which reads as a typo. The only other gap here is before 気.
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(
            best_split(measure, "元気。"),
            Some(("元".to_string(), "気。".to_string()))
        );
    }

    #[test]
    fn latin_still_breaks_only_at_spaces() {
        // The CJK rule must not leak into Latin: "HALBZEIT" has no space, so it
        // stays unsplittable and shrinks instead.
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(best_split(measure, "HALBZEIT"), None);
    }

    #[test]
    fn a_reserved_value_claims_its_width_whatever_is_being_shown() {
        // The keypad's entered value: its box must be the width of the longest
        // value the page can hold, not of the digits typed so far. A box that
        // tracked the digits would hand width back to the label between
        // keystrokes, and the label would re-fit itself every time.
        let shared = vec!["999999".to_string()];
        assert_eq!(shared_width(ruler, 38.0, &shared, false, 283.0), 114.0);
        // Nothing named, nothing claimed: every other caller is unaffected.
        assert_eq!(shared_width(ruler, 38.0, &[], false, 283.0), 0.0);
        // The widest governs, as it does for the size.
        let pair = vec!["9".to_string(), "999999".to_string()];
        assert_eq!(shared_width(ruler, 38.0, &pair, false, 283.0), 114.0);
    }

    #[test]
    fn a_wrapping_label_reserves_the_room_it_would_actually_be_drawn_in() {
        // "AB CD EF" is 8 characters on one line, 5 in the wider half once
        // wrapped. `fit_layout` draws the fewest lines that fit, so which of the
        // two the box must hold depends on the room there is -- and reserving
        // the other one moves the box as soon as the text becomes that value.
        let shared = vec!["AB CD EF".to_string()];
        // Room for one line: one line is what will be drawn, so that is what is
        // claimed. Reserving the split width here would under-reserve by 30.
        assert_eq!(shared_width(ruler, 20.0, &shared, true, 100.0), 80.0);
        // Not room for one line: it will be drawn wrapped, and only the wider
        // half has to fit.
        assert_eq!(shared_width(ruler, 20.0, &shared, true, 60.0), 50.0);
        // A label that may not wrap holds its whole line either way.
        assert_eq!(shared_width(ruler, 20.0, &shared, false, 60.0), 80.0);
        // The keypad's digits have nowhere to break, so none of this reaches
        // them: `best_split` finds no gap and the whole width stands.
        let digits = vec!["999999".to_string()];
        assert_eq!(shared_width(ruler, 20.0, &digits, true, 40.0), 60.0);
    }

    #[test]
    fn a_value_outgrowing_its_reserved_room_shrinks_instead_of_widening_the_box() {
        use crate::app::theme::MEDIUM_TEXT;

        // The GameNumber keypad reserves four digits, but `next_game_number`
        // increments without a cap: a manual 9999 becomes "10000" and the page
        // opens on five. Fitted to its reserved room those digits shrink. Fitted
        // to the whole row they would keep full size and the box would widen by
        // a digit, taking that width from the label -- and Indonesian "NOMOR /
        // PERTANDINGAN:" cannot give any up, so it would shrink past its floor
        // and clip.
        let reserved = shared_width(
            digit_ruler,
            MEDIUM_TEXT,
            &["9999".to_string()],
            false,
            f32::INFINITY,
        );
        let (_, size) = fit_layout(
            digit_ruler,
            reserved,
            &size_ladder(MEDIUM_TEXT, MIN_FIT_TEXT),
            &one("10000"),
        );
        assert!(
            size < MEDIUM_TEXT,
            "five digits should shrink into four digits of reserved room"
        );
        assert!(digit_ruler("10000", size) <= reserved);
    }

    #[test]
    fn a_fixed_share_of_the_keypad_row_was_too_narrow_for_the_longest_code() {
        use crate::app::theme::{MEDIUM_TEXT, SPACING};
        use crate::app::view_builders::keypad_pages::PANEL_ROW_WIDTH;

        // Two fifths of the keypad's title row went to the value. A six-digit
        // portal login code -- the longest that field accepts -- does not fit
        // that at `MEDIUM_TEXT`, so it shrank on the sixth keystroke: 130px of
        // digits into a 113px box.
        let ladder = size_ladder(MEDIUM_TEXT, MIN_FIT_TEXT);
        let row = PANEL_ROW_WIDTH;
        let code = "9".repeat(6);
        let old_share = row * 2.0 / 5.0;
        // The boundary the bug sat on: five digits fitted that share, six did
        // not, so the size changed on the sixth keystroke and nowhere else.
        assert!(fit_layout(digit_ruler, old_share, &ladder, &one("99999")).1 == MEDIUM_TEXT);
        assert!(fit_layout(digit_ruler, old_share, &ladder, &one(&code)).1 < MEDIUM_TEXT);
        // What is reserved instead is exactly the room those six digits need --
        // no more, so the label keeps the rest, and no less, so they never
        // shrink. Both halves of that have to hold: a reservation that drifted
        // either way would pass a "does it fit" check alone.
        let reserved = shared_width(
            digit_ruler,
            MEDIUM_TEXT,
            std::slice::from_ref(&code),
            false,
            f32::INFINITY,
        );
        assert_eq!(reserved, digit_ruler(&code, MEDIUM_TEXT));
        assert!(
            reserved > old_share,
            "the reservation must exceed the old share"
        );
        // And it leaves the label the larger half of the row.
        assert!(row - SPACING - reserved > reserved * 0.9);
    }

    #[test]
    fn splitting_a_single_word_is_not_possible() {
        let measure = |line: &str| line.chars().count() as f32;
        assert_eq!(best_split(measure, "VERWARNUNG"), None);
    }
}
