//! The Parts row of the Advanced block: the label, one chip per part, Add part and the trash
//! (ux-notes section 3).

use std::time::Duration;

use eframe::egui::{
    self, Align2, Color32, EventFilter, Key, Modifiers, Pos2, Rect, Role, Sense, Shape, Stroke, Ui,
    WidgetInfo, pos2, vec2,
};
use tracyo_core::parts::MAX_PARTS;

use crate::app::App;
use crate::bar::{bar_id, plain};
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, Side, focus_visible, radius, ring};

/// The slot of the label "Parts".
const LABEL_W: f32 = 32.0;
/// The trash arms for this long at most.
const ARM_LIFE: Duration = Duration::from_secs(4);
/// The second press removes only after this long.
const ARM_DELAY: Duration = Duration::from_millis(400);

/// The soft red of the armed group.
const DESTRUCTIVE_SOFT: Color32 = Color32::from_rgb(0xF7, 0xD3, 0xCF);
/// The armed trash.
const DESTRUCTIVE: Color32 = Color32::from_rgb(0xB3, 0x26, 0x1E);
/// A dimmed glyph: the chrome colour at 50 % on the bar.
const DIM: Color32 = Color32::from_rgb(0x8B, 0x8B, 0x8F);
/// The dashed border of a muted chip: the chrome colour at 60 % on the bar.
const MUTED_BORDER: Color32 = Color32::from_rgb(0x7B, 0x7B, 0x7F);
/// White at 50 % on the accent: the number of a muted, active chip.
const MUTED_ON_ACCENT: Color32 = Color32::from_rgb(0x97, 0xB7, 0xF7);

/// The width of the group for `parts` parts: label, chips, Add part and the trash, 4 px apart.
#[must_use]
pub(crate) fn width(parts: usize) -> f32 {
    // invariant: at most 8 parts.
    #[allow(clippy::cast_precision_loss)]
    let n = parts as f32;
    LABEL_W + 4.0 + n * 28.0 + (n - 1.0) * 4.0 + 4.0 + 28.0 + 4.0 + 28.0
}

/// The gap between the trash and the Mask group; the divider stands 12 px into it.
const GROUP_GAP: f32 = 25.0;
/// Where the row's items sit for `parts` parts in `available` px.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Layout {
    /// Height of the row: 36 px, or 72 with the Mask group on a second line.
    pub height: f32,
    /// The Mask group's top-left corner from the row's own, in px.
    pub mask: Pos2,
    /// Whether the divider between the groups is drawn (not at the start of a line).
    pub divider: bool,
}

/// The gap before the Parts row when it follows Analyse on its line; the divider stands 12 px into
/// it, as between the trash and the Mask group.
pub(crate) const LEAD_GAP: f32 = GROUP_GAP;

/// The row's width on one line: the Parts group, the gap and the Mask group.
#[must_use]
pub(crate) fn single_width(parts: usize) -> f32 {
    width(parts) + GROUP_GAP + crate::mask_group::WIDTH
}

/// The row for `parts` parts in `available` px (ux-notes 3 "Fit and wrap"): one line when the
/// Parts group, the gap and the Mask group fit, else the Mask group, as a whole, on a second line.
#[must_use]
pub(crate) fn layout(parts: usize, available: f32) -> Layout {
    let group = width(parts);
    if single_width(parts) <= available {
        Layout {
            height: 36.0,
            mask: Pos2::new(group + GROUP_GAP, 0.0),
            divider: true,
        }
    } else {
        Layout {
            height: 72.0,
            mask: Pos2::new(0.0, 36.0),
            divider: false,
        }
    }
}

/// What a chip shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mark {
    /// A muted chip.
    None,
    /// Inside: a solid square.
    Solid,
    /// Outside: an outlined square.
    Outlined,
}

/// What the maker did to the row this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Select(usize),
    Add,
    ArmOrRemove,
}

impl App {
    /// Paints and drives the row with its top-left corner at `origin`, `available` px wide: the
    /// Parts group and the Mask group; `lead_divider` draws the divider before the Parts group.
    pub(crate) fn parts_row(
        &mut self,
        ui: &mut Ui,
        ctx: &egui::Context,
        origin: Pos2,
        available: f32,
        lead_divider: bool,
    ) {
        let Some(live) = self.live_part() else { return };
        let Some(doc) = &mut self.doc else { return };
        doc.parts.store(live);
        let parts: Vec<(Mark, bool)> = doc
            .parts
            .all()
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let muted = i > 0 && p.regions.is_empty();
                // The mark is the mode, whole image or not (px-08 shows it on part 1); a muted
                // chip has none.
                let mark = match (muted, p.mode) {
                    (true, _) => Mark::None,
                    (false, tracyo_core::regions::MaskMode::Inside) => Mark::Solid,
                    (false, tracyo_core::regions::MaskMode::Outside) => Mark::Outlined,
                };
                (mark, muted)
            })
            .collect();
        let active = doc.parts.active_index();
        let count = parts.len();
        let names: Vec<String> = (0..count).map(|i| doc.parts.chip_name(i)).collect();
        let now = self.now();

        self.disarm_rules(ui, ctx, origin, count, now);
        let armed = self.trash.is_some();

        // The armed group: label to trash on the soft red, easing over 120 ms.
        // The group's own box is the whole 36 px line (the controls sit 4 px inside it).
        let whole = Rect::from_min_size(origin - vec2(0.0, 4.0), vec2(width(count), 36.0));
        let mix =
            ctx.animate_value_with_time(bar_id("parts-armed"), f32::from(u8::from(armed)), 0.12);
        if mix > 0.0 {
            ui.painter().rect_filled(
                whole,
                radius(theme::RADIUS_BUTTON),
                theme::TOOLBAR_BG.lerp_to_gamma(DESTRUCTIVE_SOFT, mix),
            );
        }
        widgets::text(
            ui,
            pos2(origin.x, origin.y + 14.0),
            Align2::LEFT_CENTER,
            "Parts",
            theme::text12(),
            theme::MENU_SECOND,
        );

        let mut action = None;
        let mut focus_in_group = false;
        for (i, (mark, muted)) in parts.iter().enumerate() {
            let rect = chip_rect(origin, i);
            let on = i == active;
            // Only the active chip is a Tab stop; the others are plain pointer targets.
            let id = bar_id("part").with(i);
            let resp = ui.interact(rect, id, if on { Sense::click() } else { Sense::hover() });
            let clicked = resp.clicked() || (!on && released_in(ui, rect));
            let name = if on {
                format!("{}, active", names[i])
            } else {
                names[i].clone()
            };
            resp.widget_info(|| WidgetInfo::selected(Role::RadioButton, true, on, name.clone()));
            paint_chip(ui, rect, i + 1, *mark, *muted, on, resp.hovered());
            widgets::tooltip(ctx, &resp, Side::Below, &plain(&names[i]));
            widgets::reset_tooltip(ctx, &resp);
            if on && resp.has_focus() {
                focus_in_group = true;
                if focus_visible(ctx) {
                    ring(ui, rect, theme::RADIUS_BUTTON, true);
                }
                if let Some(a) = self.chip_keys(ui, i, count, now) {
                    action = Some(a);
                }
            }
            if clicked && !on {
                action = Some(Action::Select(i));
                // A press hands the keyboard back to the canvas.
                if !focus_visible(ctx) {
                    ctx.memory_mut(|m| m.surrender_focus(id));
                }
            }
        }
        self.trash_focus(focus_in_group);

        // Add part.
        let can_add = count < MAX_PARTS;
        let add_rect = Rect::from_min_size(
            origin + vec2(LABEL_W + 4.0 + 32.0 * count as f32, 0.0),
            vec2(28.0, 28.0),
        );
        let add = ui
            .scope(|ui| {
                if !can_add {
                    ui.disable();
                }
                dimmed_button(
                    ui,
                    &mut self.icons,
                    add_rect,
                    bar_id("add-part"),
                    Icon::Plus,
                    "Add part",
                    can_add,
                )
            })
            .inner;
        widgets::tooltip(
            ctx,
            &add,
            Side::Below,
            &plain(if can_add {
                "Add part"
            } else {
                "8 parts at most"
            }),
        );
        widgets::reset_tooltip(ctx, &add);
        widgets::release_focus_after_mouse(ctx, &add);
        if can_add && add.clicked() {
            action = Some(Action::Add);
        }

        // The trash: no Tab stop (the focus rule of ux-notes section 2).
        let can_remove = count > 1;
        let trash_rect = Rect::from_min_size(add_rect.min + vec2(32.0, 0.0), vec2(28.0, 28.0));
        let name = if !can_remove {
            "Keep at least one part".to_owned()
        } else if armed {
            format!("Press again to remove part {}", active + 1)
        } else {
            format!("Remove part {}", active + 1)
        };
        let trash_id = bar_id("remove-part");
        let trash = ui.interact(trash_rect, trash_id, Sense::hover());
        trash.widget_info(|| WidgetInfo::labeled(Role::Button, can_remove, name.clone()));
        let hot = trash.hovered() && can_remove;
        let glow =
            ctx.animate_value_with_time(bar_id("trash-armed"), f32::from(u8::from(armed)), 0.12);
        paint_trash(ui, &mut self.icons, trash_rect, can_remove, hot, glow);
        widgets::tooltip(ctx, &trash, Side::Below, &plain(&name));
        widgets::reset_tooltip(ctx, &trash);
        if can_remove && released_in(ui, trash_rect) {
            action = Some(Action::ArmOrRemove);
        }

        match action {
            Some(Action::Select(i)) => self.select_part(ctx, i),
            Some(Action::Add) => self.add_part(ctx),
            Some(Action::ArmOrRemove) => self.arm_or_remove(ctx, now),
            None => {}
        }

        if lead_divider {
            let x = origin.x - LEAD_GAP + 12.0;
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(x, origin.y + 4.0), vec2(1.0, 20.0)),
                0,
                theme::DIVIDER,
            );
        }
        // The Mask group, after a divider (Curvyo's bar divider, 12 px into the gap).
        let places = layout(count, available);
        if places.divider {
            let x = origin.x + width(count) + 12.0;
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(x, origin.y + 4.0), vec2(1.0, 20.0)),
                0,
                theme::DIVIDER,
            );
        }
        self.mask_group(ui, ctx, origin + places.mask.to_vec2());
    }

    /// The keys of the focused active chip `i`: arrows, Home and End pick a part, Delete and
    /// Enter work the trash.
    fn chip_keys(&mut self, ui: &mut Ui, i: usize, count: usize, now: Duration) -> Option<Action> {
        ui.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        horizontal_arrows: true,
                        ..Default::default()
                    },
                );
            }
        });
        let target = ui.input_mut(|inp| {
            if inp.consume_key(Modifiers::NONE, Key::ArrowRight) {
                Some((i + 1).min(count - 1))
            } else if inp.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                Some(i.saturating_sub(1))
            } else if inp.consume_key(Modifiers::NONE, Key::Home) {
                Some(0)
            } else if inp.consume_key(Modifiers::NONE, Key::End) {
                Some(count - 1)
            } else {
                None
            }
        });
        if let Some(t) = target {
            if t != i {
                ui.memory_mut(|m| m.request_focus(bar_id("part").with(t)));
                return Some(Action::Select(t));
            }
            return None;
        }
        let delete = ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Delete));
        let enter = self.trash.is_some()
            && ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, Key::Enter));
        let _ = now;
        (count > 1 && (delete || enter)).then_some(Action::ArmOrRemove)
    }

    /// The first press arms the trash; the next after 400 ms removes the part.
    fn arm_or_remove(&mut self, ctx: &egui::Context, now: Duration) {
        match self.trash {
            None => {
                self.trash = Some(now);
                ctx.request_repaint_after(ARM_LIFE + Duration::from_millis(20));
            }
            Some(since) if now.saturating_sub(since) >= ARM_DELAY => self.remove_part(ctx),
            Some(_) => {}
        }
    }

    /// The arm ends after 4 s and on a press anywhere but the trash.
    fn disarm_rules(
        &mut self,
        ui: &Ui,
        ctx: &egui::Context,
        origin: Pos2,
        count: usize,
        now: Duration,
    ) {
        let Some(since) = self.trash else { return };
        if now.saturating_sub(since) >= ARM_LIFE {
            self.trash = None;
            return;
        }
        let trash = Rect::from_min_size(
            origin + vec2(LABEL_W + 4.0 + 32.0 * count as f32 + 32.0, 0.0),
            vec2(28.0, 28.0),
        );
        let pressed_elsewhere = ui.input(|i| {
            i.pointer.any_pressed() && i.pointer.interact_pos().is_none_or(|p| !trash.contains(p))
        });
        if pressed_elsewhere {
            self.trash = None;
        } else {
            ctx.request_repaint_after(
                ARM_LIFE.saturating_sub(now.saturating_sub(since)) + Duration::from_millis(20),
            );
        }
    }

    /// The arm ends when the keyboard focus leaves the chips it came from.
    fn trash_focus(&mut self, focus_in_group: bool) {
        if self.trash_focus_was && !focus_in_group {
            self.trash = None;
        }
        self.trash_focus_was = focus_in_group;
    }
}

fn chip_rect(origin: Pos2, i: usize) -> Rect {
    // invariant: at most 8 chips.
    #[allow(clippy::cast_precision_loss)]
    let x = LABEL_W + 4.0 + 32.0 * i as f32;
    Rect::from_min_size(origin + vec2(x, 0.0), vec2(28.0, 28.0))
}

/// Whether the primary button was released inside `rect` this frame.
fn released_in(ui: &Ui, rect: Rect) -> bool {
    ui.input(|i| i.pointer.primary_released()) && ui.rect_contains_pointer(rect)
}

/// One chip: ground, number, mark and the dashed border of a muted one.
fn paint_chip(
    ui: &Ui,
    rect: Rect,
    number: usize,
    mark: Mark,
    muted: bool,
    active: bool,
    hover: bool,
) {
    let corner = radius(theme::RADIUS_BUTTON);
    let painter = ui.painter();
    let (ground, text, mark_colour) = if active {
        (
            Some(theme::ACCENT),
            if muted {
                MUTED_ON_ACCENT
            } else {
                Color32::WHITE
            },
            Color32::WHITE,
        )
    } else if muted {
        (
            hover.then_some(theme::HOVER_ON_BAR),
            DIM,
            theme::TOOLBAR_ICON,
        )
    } else {
        (
            hover.then_some(theme::HOVER_ON_BAR),
            theme::TOOLBAR_ICON,
            theme::TOOLBAR_ICON,
        )
    };
    if let Some(ground) = ground {
        painter.rect_filled(rect, corner, ground);
    }
    if muted && !active {
        painter.extend(Shape::dashed_line(
            &rounded_outline(rect.shrink(0.5), 7.5),
            Stroke::new(1.0, MUTED_BORDER),
            3.0,
            3.0,
        ));
    }
    widgets::text(
        ui,
        rect.center(),
        Align2::CENTER_CENTER,
        &number.to_string(),
        theme::text14(),
        text,
    );
    let square = Rect::from_min_size(
        rect.right_bottom() - vec2(2.0 + 6.0, 2.0 + 6.0),
        vec2(6.0, 6.0),
    );
    match mark {
        Mark::None => {}
        Mark::Solid => {
            painter.rect_filled(square, 0, mark_colour);
        }
        Mark::Outlined => {
            painter.rect_stroke(
                square,
                0,
                Stroke::new(1.0, mark_colour),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// The closed outline of a rounded rectangle as a polyline, starting where the top-left corner's
/// arc ends.
fn rounded_outline(rect: Rect, r: f32) -> Vec<Pos2> {
    const STEPS: usize = 6;
    let mut points = Vec::with_capacity(4 * (STEPS + 1) + 1);
    let corners = [
        (
            pos2(rect.right() - r, rect.top() + r),
            -std::f32::consts::FRAC_PI_2,
        ),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (
            pos2(rect.left() + r, rect.bottom() - r),
            std::f32::consts::FRAC_PI_2,
        ),
        (pos2(rect.left() + r, rect.top() + r), std::f32::consts::PI),
    ];
    points.push(pos2(rect.left() + r, rect.top()));
    for (centre, start) in corners {
        for k in 0..=STEPS {
            // invariant: STEPS is 6.
            #[allow(clippy::cast_precision_loss)]
            let a = start + std::f32::consts::FRAC_PI_2 * k as f32 / STEPS as f32;
            points.push(centre + vec2(a.cos(), a.sin()) * r);
        }
    }
    points.push(points[0]);
    points
}

/// An icon button that can be dimmed: at half strength it has no hover ground.
fn dimmed_button(
    ui: &mut Ui,
    icons: &mut crate::icons::Icons,
    rect: Rect,
    id: egui::Id,
    icon: Icon,
    name: &str,
    enabled: bool,
) -> egui::Response {
    if enabled {
        return widgets::icon_button(ui, icons, rect, id, icon, name, false);
    }
    let resp = ui.interact(rect, id, Sense::hover());
    widgets::labelled(&resp, Role::Button, name);
    icons.paint(ui, rect, icon, 16.0, 1.5, DIM);
    resp
}

/// The trash: a plain icon button; dimmed with one part; armed, a filled red square with a white
/// glyph (`glow` is how far the arm has eased in).
fn paint_trash(
    ui: &mut Ui,
    icons: &mut crate::icons::Icons,
    rect: Rect,
    enabled: bool,
    hover: bool,
    glow: f32,
) {
    let corner = radius(theme::RADIUS_BUTTON);
    if !enabled {
        icons.paint(ui, rect, Icon::Trash, 16.0, 1.5, DIM);
        return;
    }
    if glow > 0.0 {
        ui.painter().rect_filled(
            rect,
            corner,
            DESTRUCTIVE_SOFT.lerp_to_gamma(DESTRUCTIVE, glow),
        );
    } else if hover {
        ui.painter().rect_filled(rect, corner, theme::HOVER_ON_BAR);
    }
    let glyph = theme::TOOLBAR_ICON.lerp_to_gamma(Color32::WHITE, glow);
    icons.paint(ui, rect, Icon::Trash, 16.0, 1.5, glyph);
}
