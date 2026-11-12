//! Pixel comparison of the Mask group and the regions with the references of
//! `specs/0002-trace-parts` (ux-notes 12.2: P1 to P6, P14, P16, P17 and C1 to C4; the Parts row
//! cases P7 to P13, P15 and P18 are in `parts.rs`, the lasso looks C5 to C8 in `lasso.rs`). Every
//! case runs in a 1280 x 800 window with Advanced open, where the Mask group stands.

// Pixel geometry converts between integers and f32 everywhere; the values are screen coordinates.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

mod common;
#[path = "common/compare.rs"]
mod compare;
#[path = "common/px.rs"]
mod px;

use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use image::RgbImage;
use px::Region;
use tracyo_app::App;
use tracyo_app::dialogs::Dialogs;

const SPEC: &str = "0002-trace-parts";
/// The window of ux-notes 12.2.
const WINDOW: f32 = 1280.0;
/// The bar with Advanced open: 1198 x 121 px.
const BAR_W: u32 = 1198;
const BAR_H: u32 = 121;

fn check(name: &str, h: &mut Harness<'_, App>, regions: &[Region]) {
    px::check_regions(SPEC, name, &h.render().unwrap(), regions);
}

fn whole_bar() -> Vec<Region> {
    vec![Region::same("bar", (0, 0), (BAR_W, BAR_H))]
}

fn fixed() -> Dialogs {
    Dialogs::Fixed {
        open: None,
        save: None,
    }
}

/// The logo with Method Brightness, as the reference script leaves it.
fn opened<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = common::harness_sized(dir, Some(common::logo()), fixed(), WINDOW);
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    h.get_by_label("Method").click();
    h.run_steps(3);
    h.get_by_label("Brightness").click();
    h.run_steps(3);
    common::wait_for_result(&mut h, 60.0);
    h.get_by_label("More trace options").click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(3);
    h
}

fn click_label(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_label(label).click();
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
}

fn corner(h: &Harness<'_, App>, x: f32, y: f32) -> Pos2 {
    h.state().picture_corner_on_screen(x, y).unwrap()
}

fn drag(h: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.run_steps(2);
    h.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
    for k in 1..=4 {
        h.event(Event::PointerMoved(from + (to - from) * (k as f32 / 4.0)));
        h.run_steps(2);
    }
    h.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(3);
}

/// Draws a region with the Rectangle tool (which stays pressed) and waits for the trace.
fn draw(h: &mut Harness<'_, App>, (x0, y0, x1, y1): (f32, f32, f32, f32)) {
    let (a, b) = (corner(h, x0, y0), corner(h, x1, y1));
    drag(h, a, b);
    common::wait_for_result(h, 60.0);
    h.run_steps(2);
}

const R1: (f32, f32, f32, f32) = (24.0, 25.0, 324.0, 235.0);
const R2: (f32, f32, f32, f32) = (344.0, 315.0, 619.0, 405.0);
const R3: (f32, f32, f32, f32) = (100.0, 450.0, 300.0, 520.0);

/// R1 and R2 drawn, the Rectangle tool released, the pointer gone.
fn with_r1_r2<'a>(dir: &std::path::Path) -> Harness<'a, App> {
    let mut h = opened(dir);
    click_label(&mut h, "Rectangle");
    draw(&mut h, R1);
    draw(&mut h, R2);
    click_label(&mut h, "Rectangle");
    // The automatic Threshold of the kept pixels arrives on its own thread.
    common::wait_until(&mut h, 30.0, |a| !a.is_analysing());
    common::wait_for_result(&mut h, 60.0);
    h.run_steps(3);
    h
}

fn pixel(image: &RgbImage, at: Pos2) -> [u8; 3] {
    image.get_pixel(at.x as u32, at.y as u32).0
}

fn near(a: [u8; 3], b: [u8; 3], tolerance: i32) -> bool {
    a.iter()
        .zip(&b)
        .all(|(x, y)| (i32::from(*x) - i32::from(*y)).abs() <= tolerance)
}

#[test]
fn p1_the_mask_group_at_rest() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    check("px-00-mask-group", &mut h, &whole_bar());
}

#[test]
fn p2_rectangle_pressed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    click_label(&mut h, "Rectangle");
    check("px-01-pick-armed", &mut h, &whole_bar());
}

#[test]
fn p3_inside_with_regions() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    let threshold = h.state().picture_values().unwrap().threshold;
    println!(
        "threshold {threshold} = {} %",
        tracyo_core::settings::percent_of_luma(threshold)
    );
    check("px-03-mask-inside-bar", &mut h, &whole_bar());
}

#[test]
fn p4_outside_hovered() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    h.get_by_label("Mask: Outside").hover();
    h.run_steps(3);
    h.set_render_cursor(false);
    h.run_steps(1);
    check("px-04-outside-hover", &mut h, &whole_bar());
}

#[test]
fn p5_clear_mask_tooltip() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    h.get_by_label("Clear mask").hover();
    h.run_steps(30);
    h.set_render_cursor(false);
    h.run_steps(1);
    // The bar; the tooltip box is looked at by eye.
    check("px-06-clear-mask-tooltip", &mut h, &whole_bar());
}

#[test]
fn p6_keyboard_focus_on_subtract() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    h.get_by_label("Mask: Inside").focus();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    check("px-07-region-tool-focus", &mut h, &whole_bar());
}

/// The bar of a render, cut at its top-left corner.
fn bar_of(h: &mut Harness<'_, App>) -> RgbImage {
    let full = compare::rgb(&h.render().unwrap());
    let (x, y) = compare::bar_origin(&full).unwrap();
    image::imageops::crop_imm(&full, x, y, BAR_W, BAR_H).to_image()
}

/// P16: Rectangle and Subtract pressed, against `px-19`, Curvyo's pressed class on a copy of its
/// button (Curvyo has no Subtract).
#[test]
fn p16_subtract_pressed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    click_label(&mut h, "Rectangle");
    click_label(&mut h, "Subtract");
    check("px-19-subtract-pressed", &mut h, &whole_bar());
}

/// P17: Lasso pressed, against `px-18`, the same class on a copy of the button.
#[test]
fn p17_lasso_pressed() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    click_label(&mut h, "Lasso");
    check("px-18-lasso-pressed", &mut h, &whole_bar());
}

/// The pressed Subtract button is the pressed Rectangle button with another glyph: the same
/// ground in the accent colour, a white line of 1 px for the minus.
#[test]
fn the_pressed_subtract_box_has_the_ground_of_the_pressed_rectangle() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    click_label(&mut h, "Rectangle");
    click_label(&mut h, "Subtract");
    let both = bar_of(&mut h);
    let accent = [0x2F, 0x6F, 0xEE];
    // Rectangle 153..181 and Subtract 217..245 of the bar, in the Parts row (y 85..113).
    let button_box = |x: u32| image::imageops::crop_imm(&both, 8 + 153 + x, 85, 28, 28).to_image();
    let (pressed_rect, pressed_minus) = (button_box(0), button_box(64));
    let in_glyph = |x: u32, y: u32| (4..24).contains(&x) && (4..24).contains(&y);
    for y in 0..28 {
        for x in 0..28 {
            if in_glyph(x, y) {
                continue;
            }
            let (a, b) = (
                pressed_rect.get_pixel(x, y).0,
                pressed_minus.get_pixel(x, y).0,
            );
            assert!(near(a, b, 24), "({x}, {y}): {a:?} vs {b:?}");
        }
    }
    let ground = |b: &RgbImage| b.pixels().filter(|p| near(p.0, accent, 24)).count();
    println!(
        "accent pixels: Subtract {}, Rectangle {}",
        ground(&pressed_minus),
        ground(&pressed_rect)
    );
    assert!(ground(&pressed_minus).abs_diff(ground(&pressed_rect)) <= 12);
    assert!(
        ground(&pressed_minus) >= 560,
        "a ground of {}",
        ground(&pressed_minus)
    );
}

/// The screen position of the picture pixel (`x`, `y`) centre.
fn centre(h: &Harness<'_, App>, x: u32, y: u32) -> Pos2 {
    h.state().picture_point_on_screen(x, y).unwrap()
}

fn render(h: &mut Harness<'_, App>) -> RgbImage {
    compare::rgb(&h.render().unwrap())
}

#[test]
fn c1_the_wash_inside_leaves_the_outside_at_12_percent() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    h.remove_cursor();
    h.run_steps(2);
    let image = render(&mut h);
    // The LOGO block inside R1 at 35 %: #2B4C9B over the veil.
    let block = pixel(&image, centre(&h, 50, 50));
    println!("block in R1: {block:02X?}");
    assert!(near(block, [0xB5, 0xC1, 0xDC], 3), "{block:02X?}");
    // The yellow triangle outside both at 12 %: #F2B705 becomes #FDF6E1; white stays white.
    let triangle = pixel(&image, centre(&h, 180, 340));
    println!("triangle: {triangle:02X?}");
    assert!(near(triangle, [0xFD, 0xF6, 0xE1], 3), "{triangle:02X?}");
    assert_eq!(pixel(&image, centre(&h, 5, 5)), [255, 255, 255]);
}

#[test]
fn c2_the_wash_outside_and_no_outline_inside_the_mask() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = with_r1_r2(dir.path());
    click_label(&mut h, "Mask: Outside");
    common::wait_for_result(&mut h, 60.0);
    h.remove_cursor();
    h.run_steps(3);
    let image = render(&mut h);
    let block = pixel(&image, centre(&h, 50, 50));
    println!("block in R1, Outside: {block:02X?}");
    assert!(near(block, [0xE6, 0xEA, 0xF3], 3), "{block:02X?}");
    // The preview outline (blue, with a white casing) is not drawn inside R1 or R2.
    for (x0, y0, x1, y1) in [R1, R2] {
        let (a, b) = (
            corner(&h, x0 + 3.0, y0 + 3.0),
            corner(&h, x1 - 3.0, y1 - 3.0),
        );
        let blue = (a.y as u32..b.y as u32)
            .flat_map(|y| (a.x as u32..b.x as u32).map(move |x| (x, y)))
            .filter(|&(x, y)| near(image.get_pixel(x, y).0, [0x2F, 0x6F, 0xEE], 24))
            .count();
        assert_eq!(blue, 0, "outline pixels inside the mask");
    }
    // Outside the mask the outline is there.
    let drawn = (0..image.height())
        .flat_map(|y| (60..image.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| y > 200 && near(image.get_pixel(x, y).0, [0x2F, 0x6F, 0xEE], 24))
        .count();
    assert!(drawn > 100, "the traced outline outside the mask: {drawn}");
}

#[test]
fn c3_the_region_line_is_black_and_0_25_mm_wide() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    click_label(&mut h, "Rectangle");
    draw(&mut h, R1);
    click_label(&mut h, "Rectangle");
    h.remove_cursor();
    h.run_steps(3);
    let image = render(&mut h);
    // R1's top edge at 100 %: one black pixel row, along its whole width.
    let top = corner(&h, 0.0, R1.1);
    let (x0, x1) = (
        corner(&h, R1.0 + 10.0, 0.0).x as u32,
        corner(&h, R1.2 - 10.0, 0.0).x as u32,
    );
    let row = top.y as u32;
    let black = |y: u32| {
        (x0..x1)
            .filter(|&x| near(image.get_pixel(x, y).0, [0, 0, 0], 24))
            .count()
    };
    println!(
        "black pixels per row around the top edge: {} {} {}",
        black(row - 1),
        black(row),
        black(row + 1)
    );
    assert!(black(row) as u32 >= (x1 - x0) * 9 / 10 || black(row - 1) as u32 >= (x1 - x0) * 9 / 10);
    let rows_black = (row - 3..row + 4)
        .filter(|&y| black(y) as u32 >= (x1 - x0) * 9 / 10)
        .count();
    assert_eq!(rows_black, 1, "a line of 1 px at 100 %");

    // At 400 % it is 4 px (3.8 rounded).
    let at = corner(&h, R1.0 + 50.0, R1.1 + 50.0);
    h.event(Event::PointerMoved(at));
    h.run_steps(2);
    h.event(Event::Zoom(4.0));
    h.run_steps(4);
    h.remove_cursor();
    h.run_steps(3);
    assert!(
        (h.state().zoom().unwrap() - 4.0).abs() < 0.01,
        "zoom {:?}",
        h.state().zoom()
    );
    let image = render(&mut h);
    let edge_x = corner(&h, R1.0, 0.0).x;
    let (y0, y1) = (at.y as u32 - 100, at.y as u32 + 400);
    let column = |x: u32| {
        (y0..y1)
            .filter(|&y| near(image.get_pixel(x, y).0, [0, 0, 0], 24))
            .count()
    };
    let width = (edge_x as u32 - 4..edge_x as u32 + 5)
        .filter(|&x| column(x) as u32 >= (y1 - y0) * 8 / 10)
        .count();
    println!("line width at 400 %: {width} px");
    assert_eq!(width, 4);
}

/// Whether an 8 x 8 square, white inside with a 1 px ring of the accent colour, has its centre
/// within a pixel of (`cx`, `cy`).
fn is_handle(image: &RgbImage, cx: i32, cy: i32) -> bool {
    let at = |x: i32, y: i32| image.get_pixel(x as u32, y as u32).0;
    let accent = [0x2F, 0x6F, 0xEE];
    [cx - 4, cx - 3].into_iter().any(|ox| {
        [cy - 4, cy - 3].into_iter().any(|oy| {
            (1..=6).all(|k| {
                near(at(ox + k, oy + 3), [255, 255, 255], 6)
                    && near(at(ox + 3, oy + k), [255, 255, 255], 6)
            }) && near(at(ox, oy + 3), accent, 24)
                && near(at(ox + 7, oy + 3), accent, 24)
                && near(at(ox + 3, oy), accent, 24)
                && near(at(ox + 3, oy + 7), accent, 24)
        })
    })
}

#[test]
fn c4_the_selected_rectangle_has_eight_handles() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    click_label(&mut h, "Rectangle");
    draw(&mut h, R1);
    click_label(&mut h, "Rectangle");
    // Select it with a click inside, then move the pointer away.
    let inside = corner(&h, 150.0, 120.0);
    h.event(Event::PointerMoved(inside));
    h.run_steps(2);
    for pressed in [true, false] {
        h.event(Event::PointerButton {
            pos: inside,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }
    assert!(h.state().selected_region().is_some());
    h.event(Event::PointerMoved(pos2(1000.0, 700.0)));
    h.run_steps(3);
    h.remove_cursor();
    h.run_steps(2);
    let image = render(&mut h);
    if let Some(dir) = std::env::var_os("LOOK_DIR") {
        image
            .save(std::path::Path::new(&dir).join("c4.png"))
            .unwrap();
    }
    let (a, b) = (corner(&h, R1.0, R1.1), corner(&h, R1.2, R1.3));
    let (mx, my) = (f32::midpoint(a.x, b.x), f32::midpoint(a.y, b.y));
    let accent = [0x2F, 0x6F, 0xEE];
    for (hx, hy) in [
        (a.x, a.y),
        (mx, a.y),
        (b.x, a.y),
        (b.x, my),
        (b.x, b.y),
        (mx, b.y),
        (a.x, b.y),
        (a.x, my),
    ] {
        let (cx, cy) = (hx.round() as i32, hy.round() as i32);
        assert!(
            is_handle(&image, cx, cy),
            "an 8 x 8 white square with an accent ring at ({cx}, {cy})"
        );
    }
    // The dashed box between the handles: accent pixels along the top edge, with gaps.
    let row = a.y.round() as u32;
    let along: Vec<bool> = ((a.x as u32 + 10)..(mx as u32 - 10))
        .map(|x| near(image.get_pixel(x, row).0, accent, 24))
        .collect();
    let on = along.iter().filter(|b| **b).count();
    println!(
        "dashed box: {on} of {} pixels in the accent colour",
        along.len()
    );
    assert!(
        on > along.len() / 3 && on < along.len() * 4 / 5,
        "dashed: {on} of {}",
        along.len()
    );
}

#[test]
fn p14_a_mask_outside_the_image_dims_save_as() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = opened(dir.path());
    // The reference shows Curvyo's automatic 65 %; the Threshold is typed to read the same.
    h.get_by_label("Threshold").focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(2);
    h.event(Event::Text("65".into()));
    h.run_steps(2);
    h.key_press(Key::Enter);
    common::wait_for_result(&mut h, 60.0);
    click_label(&mut h, "Rectangle");
    draw(&mut h, R3);
    click_label(&mut h, "Rectangle");
    assert_eq!(
        h.state().nothing_to_save(),
        Some("0 paths: the mask is outside the image.")
    );
    h.get_by_label("Save as\u{2026}").hover();
    h.run_steps(30);
    h.set_render_cursor(false);
    h.run_steps(1);
    // The bar without the tooltip of Save as…, which the window of this test clamps where
    // Curvyo's does not: the first row, the Output line left of the tooltip and the Parts row.
    let regions = [
        Region::same("row", (0, 0), (BAR_W, 40)),
        Region::same("output", (0, 40), (850, 32)),
        Region::same("parts", (0, 72), (BAR_W, 49)),
    ];
    check("px-16-mask-outside-image", &mut h, &regions);
}
