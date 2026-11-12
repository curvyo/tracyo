//! The part of the picture the active part does not trace, as disjoint rectangles, for the extra
//! wash of the preview.

use crate::regions::{MaskMode, PxRect, Region};

/// The rectangles of `picture` that a trace of `regions` leaves out: outside the mask when the mode
/// is Inside, inside it when the mode is Outside. No region means no extra wash (the whole picture
/// is traced). The rectangles do not overlap; the same input gives the same list.
#[must_use]
pub fn not_traced(regions: &[Region], mode: MaskMode, picture: PxRect) -> Vec<PxRect> {
    if regions.is_empty() {
        return Vec::new();
    }
    let clamp = |v: i32, lo: i32, hi: i32| v.clamp(lo, hi);
    let mut xs = vec![picture.x0, picture.x1];
    let mut ys = vec![picture.y0, picture.y1];
    for r in regions {
        xs.push(clamp(r.rect.x0, picture.x0, picture.x1));
        xs.push(clamp(r.rect.x1, picture.x0, picture.x1));
        ys.push(clamp(r.rect.y0, picture.y0, picture.y1));
        ys.push(clamp(r.rect.y1, picture.y0, picture.y1));
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let column = |x: i32| xs.partition_point(|&v| v < clamp(x, picture.x0, picture.x1));
    // The runs that continue from the strip above: (x0, x1, y0).
    let mut open: Vec<(i32, i32, i32)> = Vec::new();
    let mut done: Vec<PxRect> = Vec::new();
    for strip in ys.windows(2) {
        let (y0, y1) = (strip[0], strip[1]);
        let mut added = vec![0_i32; xs.len() + 1];
        let mut taken = vec![0_i32; xs.len() + 1];
        for r in regions {
            if r.rect.y0 <= y0 && r.rect.y1 >= y1 {
                let (a, b) = (column(r.rect.x0), column(r.rect.x1));
                let diff = if r.subtract { &mut taken } else { &mut added };
                diff[a] += 1;
                diff[b] -= 1;
            }
        }
        let mut runs: Vec<(i32, i32)> = Vec::new();
        let (mut add, mut take) = (0, 0);
        for i in 0..xs.len() - 1 {
            add += added[i];
            take += taken[i];
            let in_mask = add > 0 && take == 0;
            let left_out = in_mask == (mode == MaskMode::Outside);
            if left_out {
                match runs.last_mut() {
                    Some(last) if last.1 == xs[i] => last.1 = xs[i + 1],
                    _ => runs.push((xs[i], xs[i + 1])),
                }
            }
        }
        let mut next_open = Vec::with_capacity(runs.len());
        for (x0, x1) in runs {
            if let Some(k) = open.iter().position(|o| o.0 == x0 && o.1 == x1) {
                next_open.push(open.remove(k));
            } else {
                next_open.push((x0, x1, y0));
            }
        }
        for (x0, x1, top) in open.drain(..) {
            done.push(PxRect {
                x0,
                y0: top,
                x1,
                y1: y0,
            });
        }
        open = next_open;
    }
    let bottom = picture.y1;
    for (x0, x1, top) in open {
        done.push(PxRect {
            x0,
            y0: top,
            x1,
            y1: bottom,
        });
    }
    done.sort_by_key(|r| (r.y0, r.x0));
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region {
            rect: PxRect::new(x0, y0, x1, y1),
            subtract: false,
        }
    }

    fn take(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region {
            subtract: true,
            ..add(x0, y0, x1, y1)
        }
    }

    fn area(rects: &[PxRect]) -> i64 {
        rects
            .iter()
            .map(|r| i64::from(r.width()) * i64::from(r.height()))
            .sum()
    }

    fn disjoint(rects: &[PxRect]) -> bool {
        rects
            .iter()
            .enumerate()
            .all(|(i, a)| rects[i + 1..].iter().all(|b| a.intersection(b).is_none()))
    }

    const PICTURE: PxRect = PxRect {
        x0: 0,
        y0: 0,
        x1: 100,
        y1: 80,
    };

    #[test]
    fn without_regions_nothing_is_washed() {
        assert_eq!(not_traced(&[], MaskMode::Inside, PICTURE), []);
        assert_eq!(not_traced(&[], MaskMode::Outside, PICTURE), []);
    }

    #[test]
    fn inside_washes_what_is_around_the_mask_and_outside_what_is_in_it() {
        let regions = [add(10, 10, 30, 30)];
        let around = not_traced(&regions, MaskMode::Inside, PICTURE);
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert_eq!(area(&around), 100 * 80 - 20 * 20);
        assert_eq!(within, [PxRect::new(10, 10, 30, 30)]);
        assert!(disjoint(&around));
    }

    #[test]
    fn a_subtracted_rectangle_is_a_hole_of_the_mask() {
        let regions = [add(0, 0, 50, 50), take(10, 10, 20, 20)];
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert_eq!(area(&within), 50 * 50 - 10 * 10);
        assert!(disjoint(&within));
        let around = not_traced(&regions, MaskMode::Inside, PICTURE);
        assert_eq!(area(&around) + area(&within), 100 * 80);
        assert!(disjoint(&around));
    }

    #[test]
    fn regions_off_the_picture_are_cut_to_it() {
        let regions = [add(-50, -50, 40, 40), add(90, 70, 500, 500)];
        let within = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert_eq!(area(&within), 40 * 40 + 10 * 10);
        let nothing = not_traced(&[add(200, 0, 300, 10)], MaskMode::Outside, PICTURE);
        assert_eq!(nothing, []);
        let all = not_traced(&[add(200, 0, 300, 10)], MaskMode::Inside, PICTURE);
        assert_eq!(
            all,
            [PICTURE],
            "an empty mask washes everything with Inside"
        );
    }

    #[test]
    fn only_subtracted_regions_make_an_empty_mask() {
        let all = not_traced(&[take(10, 10, 20, 20)], MaskMode::Inside, PICTURE);
        assert_eq!(all, [PICTURE]);
        assert_eq!(
            not_traced(&[take(10, 10, 20, 20)], MaskMode::Outside, PICTURE),
            []
        );
    }

    #[test]
    fn overlapping_regions_count_once_and_the_order_does_not_matter() {
        let a = [add(0, 0, 40, 40), add(20, 20, 60, 60), take(30, 30, 35, 35)];
        let b = [a[2], a[1], a[0]];
        let one = not_traced(&a, MaskMode::Outside, PICTURE);
        assert_eq!(one, not_traced(&b, MaskMode::Outside, PICTURE));
        assert_eq!(area(&one), 1600 + 1600 - 400 - 25);
        assert!(disjoint(&one));
    }

    #[test]
    fn many_rectangles_stay_disjoint_and_complete() {
        let mut state = 12_345_u64;
        let mut next = |n: i32| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            i32::try_from((state >> 33) % u64::try_from(n).unwrap()).unwrap()
        };
        let regions: Vec<Region> = (0..40)
            .map(|i| Region {
                rect: PxRect::new(
                    next(140) - 20,
                    next(120) - 20,
                    next(140) - 20,
                    next(120) - 20,
                ),
                subtract: i % 4 == 3,
            })
            .collect();
        let inside = not_traced(&regions, MaskMode::Inside, PICTURE);
        let outside = not_traced(&regions, MaskMode::Outside, PICTURE);
        assert!(disjoint(&inside) && disjoint(&outside));
        assert_eq!(area(&inside) + area(&outside), 100 * 80);
    }
}
