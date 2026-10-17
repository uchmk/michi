//! The egui side of the short animations (#88): how far along the fade and
//! slide of a view or an overlay is. The curve and the switch are
//! `michi_core::motion`; this only remembers when each thing appeared.

use egui::{Context, Id};
use michi_core::motion::{continues, Motion};

/// Where a thing is in its fade-in, 0.0 (just appeared) to 1.0 (arrived),
/// asked once a frame while it is drawn. It starts again when it was not
/// drawn in the pass before (it was closed and opened, or another view was
/// shown). Asks for another frame while it moves.
pub fn enter(ctx: &Context, motion: Motion, id: Id) -> f32 {
    if !motion.is_on() {
        return 1.0;
    }
    let (now, pass) = (ctx.input(|i| i.time), ctx.cumulative_pass_nr());
    let (start, _) = ctx.data_mut(|d| {
        let slot = d.get_temp_mut_or_insert_with::<(f64, u64)>(id, || (now, u64::MAX));
        let last = (slot.1 != u64::MAX).then_some(slot.1);
        if !continues(last, pass) {
            slot.0 = now;
        }
        slot.1 = pass;
        *slot
    });
    let p = motion.progress(now - start);
    if p < 1.0 {
        ctx.request_repaint();
    }
    p
}

/// The pixels an overlay still has to slide (`sign` -1.0: it comes down
/// from above, 1.0: it comes up from below), and the fade to draw it with.
pub fn overlay(ctx: &Context, motion: Motion, id: Id, sign: f32) -> (f32, f32) {
    let p = enter(ctx, motion, id);
    (sign * motion.offset(p), p)
}

/// egui's own animations (a window fading in, the hover blends) last as
/// long as ours, or not at all when the motion is off. Cheap to call every frame.
pub fn apply(ctx: &Context, motion: Motion) {
    let t = motion.animation_time();
    if (ctx.global_style().animation_time - t).abs() > f32::EPSILON {
        ctx.all_styles_mut(|s| s.animation_time = t);
    }
}
