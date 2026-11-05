// Keys on the settings: typing goes into the search box, Escape leaves it (or the settings).
fn cinematic_options_key(
    game: &mut crate::game::Game<'_>,
    key: KeyCode,
    text: Option<&str>,
) -> ::core::ops::ControlFlow<Result<()>> {
    use horizon_client_lib::native_cinematic_menu as menu;
    let _ = &game;
    if !menu::is_open() {
        return ::core::ops::ControlFlow::Continue(());
    }
    menu::key(key == KeyCode::Escape, key == KeyCode::Backspace, text);
    ::core::ops::ControlFlow::Break(Ok(()))
}
