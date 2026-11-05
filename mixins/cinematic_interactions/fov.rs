// Camera.calculateFov: at the table the view zooms in a little, so the table fills the screen.
fn cinematic_fov(game: &crate::game::Game<'_>, partial: f32, camera: [f64; 3]) -> ::core::ops::ControlFlow<f32> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::shown() || ci::enter(8) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let live = game.fov_degrees(partial, camera);
    ci::leave(8);
    ::core::ops::ControlFlow::Break(ci::scene_fov(live, std::time::Instant::now()))
}
