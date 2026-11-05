// Enter in the anvil's "New name" box: the name is kept and the box closes.
fn cinematic_name_key(game: &mut crate::game::Game<'_>, key: winit::keyboard::KeyCode) -> ::core::ops::ControlFlow<bool> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if ci::renaming() && matches!(key, winit::keyboard::KeyCode::Enter | winit::keyboard::KeyCode::NumpadEnter) {
        ci::set_renaming(false);
        if let Some(st) = game.station.as_mut() {
            st.rename.focused = false;
        }
        return ::core::ops::ControlFlow::Break(true);
    }
    ::core::ops::ControlFlow::Continue(())
}
