use crate::native_cinematic_interactions as ci;
use glam::{DVec3, Vec3};
use std::time::{Duration, Instant};

const T: [i32; 3] = [10, 64, -5];

fn reset() {
    ci::set_active(false);
    ci::set_active(true);
    ci::set_config(ci::DEFAULT);
}

#[test]
fn view_looks_along_forward() {
    for yaw in [0.0f32, 90.0, 180.0, 270.0, 37.0] {
        let d = ci::ray((DVec3::ZERO, yaw, 0.0), (0.0, 0.0), 70.0, 1.6);
        assert!((d - ci::forward(yaw)).length() < 1e-4, "yaw {yaw}: {d:?}");
        // The right of the screen is the right of the view.
        let r = ci::ray((DVec3::ZERO, yaw, 0.0), (0.9, 0.0), 70.0, 1.6);
        assert!(r.dot(ci::right(yaw)) > 0.3, "yaw {yaw}");
    }
    // Pitch down looks down.
    assert!(ci::ray((DVec3::ZERO, 0.0, 45.0), (0.0, 0.0), 70.0, 1.6).y < -0.6);
}

#[test]
fn every_cell_is_where_it_is_drawn() {
    for yaw in [0.0f32, 90.0, 180.0, 270.0] {
        let pose = ci::scene_pose(T, yaw);
        for i in 0..9 {
            let n = ci::project(pose, ci::cell_center(T, yaw, i), ci::FOV, 1.86).unwrap();
            assert_eq!(ci::hover_with(pose, T, yaw, n, ci::FOV, 1.86, false, 0.0), Some(ci::Hover::Cell(i)), "yaw {yaw} cell {i}");
            // Column 0 on the left of the screen, row 0 at the top.
            if i % 3 == 0 { assert!(n.0 < -0.1); }
            if i % 3 == 2 { assert!(n.0 > 0.1); }
            if i / 3 == 0 { assert!(n.1 > ci::project(pose, ci::cell_center(T, yaw, 6), ci::FOV, 1.86).unwrap().1); }
        }
        // Off the table: nothing.
        assert_eq!(ci::hover_with(pose, T, yaw, (0.98, -0.98), ci::FOV, 1.86, false, 0.0), None);
    }
}

#[test]
fn the_table_fills_the_view_as_in_the_scene() {
    let pose = ci::scene_pose(T, 0.0);
    let top = ci::top_center(T);
    let f = ci::forward(0.0);
    let far = ci::project(pose, top + f * 0.5, ci::FOV, 1.86).unwrap();
    let near = ci::project(pose, top - f * 0.5, ci::FOV, 1.86).unwrap();
    // Screen y from the top, 0..1.
    let (far_y, near_y) = ((1.0 - far.1) / 2.0, (1.0 - near.1) / 2.0);
    println!("far edge {far_y:.2}, near edge {near_y:.2}");
    assert!((0.3..0.45).contains(&far_y), "{far_y}");
    assert!((0.7..0.9).contains(&near_y), "{near_y}");
}

#[test]
fn the_player_walks_up_leans_over_and_glides_back() {
    reset();
    let t0 = Instant::now();
    let ms = |m: u64| t0 + Duration::from_millis(m);
    let live = Some((DVec3::new(10.5, 65.62, -9.5), 0.0f32, 20.0f32));
    let player = Some(DVec3::new(10.5, 64.0, -9.5));
    ci::clicked(T, false, t0);
    let p = ci::camera(true, None, live, player, 3.0, t0).unwrap();
    assert!((p.0 - live.unwrap().0).length() < 1e-6, "starts at the eye");
    assert!(ci::walking(t0) && ci::controls());
    // The walk: 3.65 blocks to the spot in front of the table, at walking speed.
    let stand = ci::stand_spot(T, 0.0, 64.0);
    assert!((stand.z - (-5.0 + 0.5 - 0.86)).abs() < 1e-9);
    let mid = ci::camera(true, None, live, player, 3.0, ms(400)).unwrap();
    assert!(mid.0.z > -9.4 && mid.0.z < stand.z, "{:?}", mid.0);
    assert!((mid.0.y - 65.62).abs() < 0.06, "the bob stays small");
    assert!(ci::bars(ms(400)) > 0.0, "the bars slide in while walking");
    assert_eq!(ci::take_arrival(ms(400)), None);
    let walk = (stand.z - -9.5) as f32 / ci::WALK_SPEED as f32 + ci::WALK_LEAD;
    let after = t0 + Duration::from_secs_f32(walk + 0.01);
    ci::camera(true, None, live, player, 3.0, after).unwrap();
    assert!(!ci::walking(after));
    let (feet, yaw) = ci::take_arrival(after).unwrap();
    assert!(feet.distance(stand) < 1e-9 && yaw == 0.0);
    assert_eq!(ci::take_arrival(after), None, "once");
    // Then it leans over the table (1.2 s), the bars full early on and gone at the end.
    let lean = after + Duration::from_millis(100);
    ci::camera(true, None, live, player, 3.0, lean).unwrap();
    assert!(ci::bars(lean) > 0.95);
    let end = ci::camera(true, None, live, player, 3.0, after + Duration::from_millis(1300)).unwrap();
    assert_eq!(ci::bars(after + Duration::from_millis(1300)), 0.0);
    assert_eq!(ci::phase(), ci::Phase::Inside);
    let sp = ci::scene_pose(T, 0.0);
    assert!(end.0.distance(sp.0) < 1e-6 && end.1 == sp.1 && end.2 == sp.2);
    // Esc: back to the eye.
    let t1 = after + Duration::from_millis(1400);
    ci::camera(false, None, live, player, 3.0, t1).unwrap();
    assert_eq!(ci::phase(), ci::Phase::Exit);
    assert_eq!(ci::camera(false, None, live, player, 3.0, t1 + Duration::from_millis(1100)), None);
    assert_eq!(ci::phase(), ci::Phase::Idle);
}

#[test]
fn right_beside_the_table_there_is_no_walk() {
    reset();
    let t0 = Instant::now();
    let stand = ci::stand_spot(T, 0.0, 64.0);
    let live = Some((stand + DVec3::Y * 1.62, 0.0f32, 30.0f32));
    ci::clicked(T, false, t0);
    ci::camera(true, None, live, Some(stand), 0.0, t0);
    assert!(!ci::walking(t0));
    assert!(ci::bars(t0 + Duration::from_millis(360)) > 0.99);
}

#[test]
fn the_recipe_panel() {
    assert_eq!(ci::book_hit(0.0, 0.0, false), None);
    let (x, y) = ci::book_icon(5);
    assert_eq!(ci::book_hit(x + 4.0, y + 4.0, false), Some(ci::BookHit::Recipe(5)));
    let b = ci::book_buttons();
    assert_eq!(ci::book_hit(b[5][0] + 5.0, b[5][1] + 2.0, true), Some(ci::BookHit::LayOut));
    assert_eq!(ci::book_hit(b[5][0] + 5.0, b[5][1] + 2.0, false), Some(ci::BookHit::Panel));
    assert_eq!(ci::book_hit(b[0][0] + 2.0, b[0][1] + 2.0, false), Some(ci::BookHit::Minimize));
    let s = ci::book_search();
    assert_eq!(ci::book_hit(s[0] + 2.0, s[1] + 2.0, false), Some(ci::BookHit::Search));
    // The last row of icons stays above the page line.
    assert!(ci::book_icon(15).1 + 8.0 < b[2][1]);
}

#[test]
fn a_sneak_click_or_the_recipe_book_shows_the_normal_screen() {
    reset();
    let t0 = Instant::now();
    let live = Some((DVec3::new(10.5, 65.6, -8.0), 0.0f32, 20.0f32));
    ci::clicked(T, true, t0);
    assert_eq!(ci::camera(true, None, live, None, 0.0, t0), None);
    assert!(!ci::controls());
    ci::camera(false, None, live, None, 0.0, t0);
    ci::clicked(T, false, t0);
    assert!(ci::camera(true, None, live, None, 0.0, t0).is_some());
    ci::go_plain();
    ci::camera(true, None, live, None, 0.0, t0 + Duration::from_millis(50));
    assert_eq!(ci::phase(), ci::Phase::Exit);
    assert!(!ci::controls());
    ci::camera(false, None, live, None, 0.0, t0 + Duration::from_secs(3));
    ci::camera(false, None, live, None, 0.0, t0 + Duration::from_secs(4));
    assert_eq!(ci::phase(), ci::Phase::Idle);
}

#[test]
fn the_table_faces_the_player() {
    // The player west of the table looks east (yaw 270).
    assert_eq!(ci::facing(Some(DVec3::new(7.5, 64.0, -4.5)), T, 0.0), 270.0);
    assert_eq!(ci::facing(Some(DVec3::new(10.5, 64.0, -9.0)), T, 0.0), 0.0);
    assert_eq!(ci::facing(Some(DVec3::new(10.5, 64.0, 0.0)), T, 0.0), 180.0);
}

#[test]
fn range_rules() {
    let at = |x: f64, y: f64, z: f64| ci::reach(DVec3::new(x, y, z), T);
    assert_eq!(at(13.5, 64.0, -5.5), None, "same level");
    assert_eq!(at(14.5, 66.0, -5.5), Some("Too low"), "two higher");
    assert_eq!(at(11.5, 65.0, -4.5), None, "one higher, beside it");
    assert_eq!(at(13.5, 65.0, -5.5), Some("Too low"), "one higher, further away");
    assert_eq!(at(10.5, 65.0, -4.5), None, "standing on it");
}

#[test]
fn an_arm_goes_from_the_shoulder_to_the_hand() {
    let bounds = ([-3.0, -2.0, -2.0], [1.0, 10.0, 2.0]);
    let (s, h) = (Vec3::new(0.4, -0.5, 0.1), Vec3::new(-0.1, -1.0, 1.2));
    let m = ci::arm_matrix(s, h, bounds);
    assert!(m.transform_point3(Vec3::new(-1.0, -2.0, 0.0) / 16.0).distance(s) < 1e-4);
    assert!(m.transform_point3(Vec3::new(-1.0, 10.0, 0.0) / 16.0).distance(h) < 1e-4);
    assert!(m.determinant() > 0.0, "faces keep their side");
    // The arm's width stays 4 pixels.
    let w = m.transform_vector3(Vec3::new(4.0 / 16.0, 0.0, 0.0)).length();
    assert!((w - 0.25).abs() < 1e-4);
}

#[test]
fn hands_rest_and_reach() {
    reset();
    let t0 = Instant::now();
    ci::clicked(T, false, t0);
    let live = Some((DVec3::new(10.5, 65.6, -8.0), 0.0f32, 20.0f32));
    ci::camera(true, None, live, None, 0.0, t0);
    ci::camera(true, None, live, None, 0.0, t0 + Duration::from_secs(3));
    let rest = ci::step_hands(None, t0 + Duration::from_secs(3), 0.0).unwrap();
    assert!(rest[0][1].distance(ci::rest(0, T, 0.0)) < 1e-6);
    // The far left cell: the left hand goes, step by step.
    let mut t = t0 + Duration::from_secs(3);
    for _ in 0..60 {
        t += Duration::from_millis(16);
        ci::step_hands(Some(ci::Hover::Cell(0)), t, 0.0);
    }
    let reached = ci::step_hands(Some(ci::Hover::Cell(0)), t, 0.0).unwrap();
    assert_eq!(ci::reaching(), Some(0));
    assert!(reached[0][1].distance(ci::cell_center(T, 0.0, 0)) < 0.13);
    assert!(reached[1][1].distance(ci::rest(1, T, 0.0)) < 0.01);
    // The middle column is the right hand's.
    assert_eq!(ci::reach_for(ci::Hover::Cell(4), T, 0.0, 0.0).0, 1);
    // The shoulders are off screen.
    let pose = ci::scene_pose(T, 0.0);
    for side in 0..2 {
        let n = ci::project(pose, ci::shoulder(side, pose, 0.0), ci::FOV, 1.86).unwrap();
        assert!(n.1 < -1.0 || n.0.abs() > 1.0, "{n:?}");
    }
}

#[test]
fn the_panel_and_its_slots() {
    let w = 480.0;
    let (x, y) = ci::slot_pos(w, 0);
    assert_eq!(ci::panel_slot(w, x + 8.0, y + 8.0, false), Some(0));
    let (x, y) = ci::slot_pos(w, 8);
    assert_eq!(ci::panel_slot(w, x + 8.0, y + 8.0, false), Some(8));
    let (x, y) = ci::slot_pos(w, 35);
    assert_eq!(ci::panel_slot(w, x + 8.0, y + 8.0, false), None, "closed");
    assert_eq!(ci::panel_slot(w, x + 8.0, y + 8.0, true), Some(35));
    let p = ci::panel(w, true);
    assert!(y + 16.0 <= p[1] + p[3]);
    // Centred like the scene's panel (it spans 159..320 at 480 wide).
    assert_eq!(ci::panel(w, false)[0], 159.0);
}

#[test]
fn fov_fix_is_nothing_at_the_same_fov() {
    let view = ci::view_of(ci::scene_pose(T, 90.0));
    let p = Vec3::new(0.3, -0.7, 0.4);
    assert!(ci::fov_fix(p, view, view.inverse(), 1.0).distance(p) < 1e-5);
    assert!(ci::fov_k(70.0, 90.0) < 1.0);
}

#[test]
fn settings_round_trip() {
    let c = ci::Config { bars: false, hints: true, speed: 2, off: 5, zoom: 120, developer: true, ..ci::DEFAULT };
    assert_eq!(ci::parse(&ci::to_text(&c)), c);
    ci::set_config(ci::DEFAULT);
    assert_eq!(ci::setting_labels()[2], "Camera speed: Normal");
}

#[test]
fn dump_preview() {
    // A picture of the scene's geometry, for a look beside the screenshots (PREVIEW=1, or
    // PREVIEW=furnace).
    let Ok(which) = std::env::var("PREVIEW") else { return };
    let (w, h) = (1920.0f32, 1009.0f32);
    let (fov, aspect) = (ci::FOV, w / h);
    let furnace = which == "furnace";
    let smithing = which == "smithing";
    let kind = if furnace { ci::Kind::Furnace } else if smithing { ci::Kind::Smithing } else { ci::Kind::Table };
    let pose = ci::pose_for(kind, T, 0.0);
    let to_px = |p: DVec3| ci::project(pose, p, fov, aspect).map(|n| [((n.0 + 1.0) * 0.5 * w), ((1.0 - n.1) * 0.5 * h)]);
    let poly = |pts: &[DVec3]| pts.iter().map(|p| format!("{:?}", to_px(*p).unwrap_or([0.0, 0.0]))).collect::<Vec<_>>().join(",");
    let mut out = String::from("{");
    let top = ci::top_center(T);
    let (r, f) = (ci::right(0.0), ci::forward(0.0));
    out += &format!("\"top\":[{}],", poly(&[top - r * 0.5 - f * 0.5, top + r * 0.5 - f * 0.5, top + r * 0.5 + f * 0.5, top - r * 0.5 + f * 0.5]));
    let mut zones = Vec::new();
    if smithing {
        for s in ci::SMITHING_SPOTS.iter() {
            let c = ci::spot_point(T, 0.0, s);
            zones.push(format!("[{}]", poly(&[c - r * s.r - f * s.r, c + r * s.r - f * s.r, c + r * s.r + f * s.r, c - r * s.r + f * s.r])));
        }
    } else if furnace {
        for z in ci::FURNACE.zones.iter() {
            let p = |u: f64, v: f64| ci::front_point(T, 0.0, u, v, 0.0);
            zones.push(format!("[{}]", poly(&[p(z.u0, z.v0), p(z.u1, z.v0), p(z.u1, z.v1), p(z.u0, z.v1)])));
        }
    } else {
        for i in 0..9 {
            let c = ci::cell_center(T, 0.0, i);
            zones.push(format!("[{}]", poly(&[c - r * 0.08 - f * 0.08, c + r * 0.08 - f * 0.08, c + r * 0.08 + f * 0.08, c - r * 0.08 + f * 0.08])));
        }
    }
    out += &format!("\"zones\":[{}],", zones.join(","));
    let mut arms = Vec::new();
    for side in 0..2 {
        let s = ci::shoulder(side, pose, 0.0);
        let hand = if furnace { ci::furnace_rest(kind, side, T, 0.0) } else if smithing { ci::surface_rest(kind, side, T, 0.0) } else { ci::rest(side, T, 0.0) };
        for q in ci::arm_quads(s, hand, side == 0, false) {
            if q.corners.iter().all(|p| to_px(*p).is_some()) {
                arms.push(format!("{{\"p\":[{}],\"s\":{},\"uv\":{:?}}}", poly(&q.corners), q.shade, q.uv[0]));
            }
        }
    }
    out += &format!("\"arms\":[{}]}}", arms.join(","));
    std::fs::write(format!("/tmp/ci_preview_{which}.json"), out).unwrap();
}

#[test]
fn the_arm_is_the_jars_long_straight_one() {
    // The shoulders sit 0.38 to each side of the eye, 0.46 below it: the right one on the right.
    let pose = ci::scene_pose(T, 0.0);
    let right = ci::shoulder(1, pose, 0.0);
    let left = ci::shoulder(0, pose, 0.0);
    assert!((right - pose.0).dot(ci::right(0.0)) > 0.37 && (left - pose.0).dot(ci::right(0.0)) < -0.37);
    assert!((pose.0.y - right.y - 0.46).abs() < 1e-9);
    // From the shoulder to the hand: the hand's end is the hand, the arm reaches 0.18 behind the
    // shoulder, and the hand's 4 pixels keep their size however far it reaches.
    let hand = ci::rest(1, T, 0.0);
    let f = ci::arm_frame(right, hand);
    assert!(ci::arm_point(&f, 0.0, 10.0, 0.0, false).distance(hand) < 1e-9);
    let back = ci::arm_point(&f, 0.0, -2.0, 0.0, true);
    assert!((back.distance(hand) - (right.distance(hand) + 0.18)).abs() < 1e-6, "{}", back.distance(hand));
    let wrist = ci::arm_point(&f, 0.0, 6.0, 0.0, false);
    assert!((wrist.distance(hand) - 4.0 * ci::ARM_SCALE / 16.0).abs() < 1e-9);
    // The arm's top faces up.
    assert!(f.z.y < -0.5, "{:?}", f.z);
    // Arm and sleeve: 4 sides for the upper part and the hand, and the hand's end, each.
    let quads = ci::arm_quads(right, hand, false, false);
    assert_eq!(quads.len(), 18);
    // The base arm's skin is the right arm's (40, 16), the sleeve's (40, 32).
    assert!(quads[..9].iter().all(|q| q.uv.iter().all(|uv| uv[0] >= 40.0 && uv[0] <= 56.0 && uv[1] >= 16.0 && uv[1] <= 32.0)));
    assert!(quads[9..].iter().all(|q| q.uv.iter().all(|uv| uv[1] >= 32.0 && uv[1] <= 48.0)));
    // The left arm's skin is at (32, 48).
    assert!(ci::arm_quads(left, ci::rest(0, T, 0.0), true, false)[..9].iter().all(|q| q.uv.iter().all(|uv| uv[0] >= 32.0 && uv[1] >= 48.0)));
}

#[test]
fn stations_have_their_own_cameras() {
    // The furnace and the smoker are worked crouching, looking down at about 32 degrees; the
    // blast furnace standing, at about 44.
    let low = ci::pose_for(ci::Kind::Furnace, T, 0.0);
    assert!((low.2 - 31.9).abs() < 0.5, "{}", low.2);
    assert!((low.0.y - (f64::from(T[1]) + 1.0)).abs() < 1e-9);
    let tall = ci::pose_for(ci::Kind::BlastFurnace, T, 0.0);
    assert!((tall.2 - 44.0).abs() < 1.5, "{}", tall.2);
    assert!((ci::pose_for(ci::Kind::Table, T, 0.0).2 - ci::PITCH).abs() < 0.01);
    assert_eq!(ci::kind_of("minecraft:anvil", false), Some(ci::Kind::Anvil));
    assert_eq!(ci::kind_of("minecraft:furnace", true), Some(ci::Kind::Furnace));
    assert_eq!(ci::kind_of("minecraft:blast_furnace", true), Some(ci::Kind::BlastFurnace));
    assert_eq!(ci::kind_of("minecraft:chest", false), None);
    // The player stands a little different from each.
    let a = ci::stand_for(ci::Kind::Anvil, T, 0.0, 64.0);
    assert!((a.z - (-5.0 + 0.5 - 0.86)).abs() < 1e-9);
    // A furnace facing north is looked at from the north (yaw 0), one facing east from the east.
    assert_eq!(ci::yaw_facing("north"), Some(0.0));
    assert_eq!(ci::yaw_facing("east"), Some(90.0));
}

#[test]
fn a_furnaces_zones_are_where_the_jar_has_them() {
    let yaw = 0.0;
    let pose = ci::pose_for(ci::Kind::Furnace, T, yaw);
    let (fov, aspect) = (ci::FOV, 16.0f32 / 9.0);
    let at = |p: DVec3| ci::project(pose, p, fov, aspect).unwrap();
    // The upper opening's left half takes what is smelted, its right half gives the result, the
    // fire box below takes the fuel.
    let pick = |u: f64, v: f64| ci::furnace_pick(pose, T, yaw, ci::Kind::Furnace, at(ci::front_point(T, yaw, u, v, 0.0)), fov, aspect);
    assert_eq!(pick(0.3, 0.7), Some(0));
    assert_eq!(pick(0.7, 0.7), Some(2));
    assert_eq!(pick(0.5, 0.2), Some(1));
    assert_eq!(pick(0.5, 0.45), None);
    // The left of the face is on the left of the screen.
    assert!(at(ci::front_point(T, yaw, 0.2, 0.5, 0.0)).0 < at(ci::front_point(T, yaw, 0.8, 0.5, 0.0)).0);
    // The blast furnace is charged from the top.
    let pose = ci::pose_for(ci::Kind::BlastFurnace, T, yaw);
    let at = |p: DVec3| ci::project(pose, p, fov, aspect).unwrap();
    assert_eq!(ci::furnace_pick(pose, T, yaw, ci::Kind::BlastFurnace, at(ci::top_point(T, yaw, 0.5, 0.6, 0.0)), fov, aspect), Some(0));
    // The hands wait low in front of a furnace, on the top of a blast furnace; the right one on the
    // right.
    let r = ci::furnace_rest(ci::Kind::Furnace, 1, T, yaw);
    let l = ci::furnace_rest(ci::Kind::Furnace, 0, T, yaw);
    assert!((r - l).dot(ci::right(yaw)) > 0.5);
    assert!((r.y - (f64::from(T[1]) + 0.09)).abs() < 1e-9);
    assert!(ci::furnace_rest(ci::Kind::BlastFurnace, 1, T, yaw).y > f64::from(T[1]) + 1.0);
    // Items lie in their beds: a block alone, flat items in layers by how many.
    assert_eq!(ci::bed_copies(64, true, 3), 1);
    assert_eq!(ci::bed_copies(64, false, 4), 4);
    assert_eq!(ci::bed_copies(20, false, 4), 3);
    assert_eq!(ci::bed_copies(1, false, 4), 1);
    assert_eq!(ci::bed_spots(&ci::FURNACE.beds[2], T, yaw, 4).len(), 4);
    assert_eq!(ci::zone_title(ci::Kind::Furnace, 1), "Fire box \u{2014} fuel");
    assert_eq!(ci::burning_text(1600), "Burns for 80 more s");
}

#[test]
fn a_drag_shares_the_stack_out() {
    assert_eq!(ci::spread_text(10, 3, false).as_deref(), Some("Shared out: 3 into each of 3 cells  \u{b7}  Left in hand: 1"));
    assert_eq!(ci::spread_text(10, 3, true).as_deref(), Some("Shared out: 1 into each of 3 cells  \u{b7}  Left in hand: 7"));
    assert_eq!(ci::spread_text(10, 1, false), None);
    assert_eq!(ci::spread_text(2, 3, false), None);
}

#[test]
fn laying_out_visits_the_cells() {
    reset();
    let t0 = Instant::now();
    let step = |cell: usize| ci::LayStep { cell, take: false, items: vec!["minecraft:oak_planks".into()], count: 1 };
    ci::start_lay(vec![step(0), step(4), step(8)], t0);
    assert!(ci::laying());
    assert_eq!(ci::lay_target(t0), Some(0));
    assert_eq!(ci::lay_target(t0 + Duration::from_millis(350)), Some(4));
    assert_eq!(ci::lay_target(t0 + Duration::from_millis(700)), Some(8));
    // Each step is taken once, when the hand has reached its cell.
    assert!(ci::take_lay_steps(t0 + Duration::from_millis(100)).is_empty());
    assert_eq!(ci::take_lay_steps(t0 + Duration::from_millis(250)), vec![step(0)]);
    assert!(ci::take_lay_steps(t0 + Duration::from_millis(300)).is_empty());
    assert_eq!(ci::take_lay_steps(t0 + Duration::from_millis(950)), vec![step(4), step(8)]);
    assert!(!ci::laying());
}

#[test]
fn the_plan_takes_back_what_is_in_the_way_first() {
    let planks = || vec!["minecraft:oak_planks".to_string(), "minecraft:birch_planks".to_string()];
    let wants = vec![(0, planks()), (1, planks()), (3, planks()), (4, planks())];
    let mut grid: Vec<Option<(String, u32)>> = vec![None; 9];
    grid[1] = Some(("minecraft:stone".into(), 1));
    grid[8] = Some(("minecraft:dirt".into(), 3));
    let stock = vec![("minecraft:birch_planks".to_string(), 10)];
    let plan = ci::lay_plan(&wants, &grid, &stock, 1);
    let cells: Vec<(usize, bool)> = plan.iter().map(|p| (p.cell, p.take)).collect();
    assert_eq!(cells, vec![(1, true), (8, true), (0, false), (1, false), (3, false), (4, false)]);
    // With Shift: as many whole sets as the stock covers.
    assert_eq!(ci::lay_sets(&wants, &stock), 2);
    assert_eq!(ci::lay_plan(&wants, &grid, &stock, 2)[2].count, 2);
    // Nothing in the stock: nothing to lay, only to take back.
    assert!(ci::lay_plan(&wants, &grid, &[], 1).iter().all(|p| p.take));
}

#[test]
fn the_recipe_list_sorts_and_searches_like_the_jar() {
    let mut list = vec![
        ("Acacia Boat with Chest".to_string(), "minecraft:acacia_chest_boat".to_string(), 1, false),
        ("Acacia Boat".to_string(), "minecraft:acacia_boat".to_string(), 0, false),
        ("Activator Rail".to_string(), "minecraft:activator_rail".to_string(), 2, false),
    ];
    ci::sort_recipes(&mut list);
    assert_eq!(list.iter().map(|l| l.2).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert!(ci::search_matches("Acacia Boat", "BOAT"));
    assert!(!ci::search_matches("Acacia Boat", "rail"));
    assert_eq!(ci::plain_name("\u{a7}bDiamond Sword"), "Diamond Sword");
}

#[test]
fn kept_tables_survive_their_file() {
    let mut items: [Option<(String, u32)>; 9] = Default::default();
    items[0] = Some(("minecraft:cobblestone".into(), 3));
    items[4] = Some(("minecraft:stick".into(), 1));
    let list = vec![ci::KeptTable { dim: "Overworld".into(), pos: [10, 64, -3], yaw: 90.0, items }];
    assert_eq!(ci::tables_from_text(&ci::tables_to_text(&list)), list);
    // An empty table isn't kept, a broken line is skipped.
    assert!(ci::tables_from_text("Overworld 1 2 3 0|-|-|-|-|-|-|-|-|-\nnonsense\n").is_empty());
}

#[test]
fn the_campfire_spots_are_the_renderers() {
    // Facing south (0): spot 0 is the renderer's first corner, all four 0.3125 out at 0.449.
    let spots: Vec<glam::DVec3> = (0..4).map(|i| ci::campfire_offset(0, i)).collect();
    for s in &spots {
        assert!((s.x.abs() - 0.3125).abs() < 1e-9 && (s.z.abs() - 0.3125).abs() < 1e-9);
        assert!((s.y - 0.44921875).abs() < 1e-9);
    }
    let mut corners: Vec<(i64, i64)> = spots.iter().map(|s| (s.x.signum() as i64, s.z.signum() as i64)).collect();
    corners.sort();
    corners.dedup();
    assert_eq!(corners.len(), 4);
    // The enchanting table's middle rune is the highest.
    let r = ci::rune_points([0, 64, 0], 0.0);
    assert!(r[1].y > r[0].y && (r[0].y - r[2].y).abs() < 1e-9);
}
