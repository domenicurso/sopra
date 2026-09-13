use std::time::{Duration, Instant};

use crate::blink::MIN_CURSOR_OPACITY;

use super::*;

#[test]
fn idle_clock_does_not_request_frames() {
    let clock = FrameClock::new();
    assert_eq!(clock.decision(Instant::now()), FrameDecision::Idle);
}

#[test]
fn typing_and_resize_render_immediately() {
    let now = Instant::now();
    let mut clock = FrameClock::new();
    clock.invalidate(InvalidationReason::Content, now);
    assert_eq!(clock.decision(now), FrameDecision::RenderNow);
    clock.rendered(now);
    clock.invalidate(InvalidationReason::Resize, now);
    assert_eq!(clock.decision(now), FrameDecision::RenderNow);
}

#[test]
fn animation_waits_until_the_next_tick() {
    let now = Instant::now();
    let mut clock = FrameClock::new();
    clock.set_editing(true, now);
    clock.set_animation_interval(Some(Duration::from_millis(100)), now);
    assert_eq!(
        clock.decision(now),
        FrameDecision::Wait(Duration::from_millis(100))
    );
    assert_eq!(
        clock.decision(now + Duration::from_millis(100)),
        FrameDecision::RenderNow
    );
}

#[test]
fn rendering_clears_pending_work_but_keeps_animation_alive() {
    let now = Instant::now();
    let mut clock = FrameClock::new();
    clock.set_editing(true, now);
    clock.set_animation_interval(Some(Duration::from_millis(50)), now);
    clock.invalidate(InvalidationReason::Animation, now);
    clock.rendered(now + Duration::from_millis(50));
    assert!(!clock.pending());
    assert_eq!(
        clock.decision(now + Duration::from_millis(50)),
        FrameDecision::Wait(Duration::from_millis(50))
    );
}

#[test]
fn animation_is_ignored_when_the_editor_is_idle() {
    let now = Instant::now();
    let mut clock = FrameClock::new();
    clock.set_animation_interval(Some(Duration::from_millis(50)), now);
    clock.invalidate(InvalidationReason::Animation, now);
    assert_eq!(clock.decision(now), FrameDecision::Idle);
}

#[test]
fn cursor_blink_has_intermediate_opacity_values() {
    let blink = CursorBlink::new(Duration::from_millis(1_000));

    assert_eq!(blink.opacity_at(Duration::ZERO), 1.0);
    assert_eq!(
        blink.opacity_at(Duration::from_millis(500)),
        MIN_CURSOR_OPACITY
    );
    assert_eq!(blink.opacity_at(Duration::from_millis(1_000)), 1.0);
    assert!(blink.opacity_at(Duration::from_millis(125)) > MIN_CURSOR_OPACITY);
    assert!(blink.opacity_at(Duration::from_millis(125)) < 1.0);
    assert!(blink.opacity_at(Duration::from_millis(625)) > MIN_CURSOR_OPACITY);
    assert!(blink.opacity_at(Duration::from_millis(625)) < 1.0);
}

#[test]
fn editing_clock_exposes_the_interpolated_cursor_opacity() {
    let now = Instant::now();
    let mut clock = FrameClock::new();
    clock.set_cursor_blink(CursorBlink::new(Duration::from_millis(1_000)));
    clock.set_editing(true, now);

    let opacity = clock.cursor_opacity(now + Duration::from_millis(125));
    assert!(opacity > 0.0 && opacity < 1.0);
}

#[test]
fn refreshing_editing_state_does_not_restart_the_cursor_animation() {
    let now = Instant::now();
    let mut clock = FrameClock::new();
    clock.set_editing(true, now);
    clock.set_editing(true, now + Duration::from_millis(125));

    let opacity = clock.cursor_opacity(now + Duration::from_millis(125));
    assert!(opacity > 0.0 && opacity < 1.0);
}
