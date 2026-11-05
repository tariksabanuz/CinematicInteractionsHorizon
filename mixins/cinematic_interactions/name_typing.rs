// The anvil in the scene: typing goes to its name only once the floating name was clicked (the
// "New name" box is open); else the keys are the scene's (a step leaves, E closes).
fn cinematic_name_typing(game: &crate::game::Game<'_>) -> ::core::ops::ControlFlow<bool> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if ci::controls() && ci::kind() == ci::Kind::Anvil && !ci::renaming() {
        return ::core::ops::ControlFlow::Break(false);
    }
    ::core::ops::ControlFlow::Continue(())
}
