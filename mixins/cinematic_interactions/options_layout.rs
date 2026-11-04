// Options' first page: the wide "Cinematic Interaction Settings..." button beside the others
// (in the free gap above them, beside a lone button, or in a new row). Its press is the mouse
// hook's (it knows where the button is from here). It runs before other mods' layout hooks and
// lays out around their buttons.
fn cinematic_options_layout(
    ui: &mut OptionsUi,
    s: &Settings,
    w: f32,
    h: f32,
) -> ::core::ops::ControlFlow<(Vec<Widget>, f32, f32, f32)> {
    use crate::native_cinematic_button as cb;
    if ui.page() != Page::Main || !cb::layout_enter() {
        return ::core::ops::ControlFlow::Continue(());
    }
    let (mut widgets, top, bottom, content) = ui.layout(s, w, h);
    cb::layout_leave();
    let rects: Vec<[f32; 3]> = widgets.iter().map(|e| [e.x, e.y, e.w]).collect();
    match cb::place(&rects, w, h) {
        Some((shift, r)) => {
            if let Some(from) = shift {
                for e in &mut widgets {
                    if e.y >= from && e.y < h - 30.0 {
                        e.y += 24.0;
                    }
                }
            }
            widgets.push(button(r[0], r[1], r[2], Text::literal(cb::LABEL), Act::None));
            cb::set_button(Some(r));
        }
        None => cb::set_button(None),
    }
    ::core::ops::ControlFlow::Break((widgets, top, bottom, content))
}
