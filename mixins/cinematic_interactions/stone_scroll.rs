// The wheel at the stonecutter: the next shapes round the blade (or the ones before), four at a time.
fn cinematic_stone_scroll(game: &mut crate::game::Game<'_>, d: f32) -> ::core::ops::ControlFlow<()> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::controls() || ci::kind() != ci::Kind::Stonecutter {
        return ::core::ops::ControlFlow::Continue(());
    }
    let total = game.station.as_ref().map_or(0, |s| s.stone.recipes.len());
    ci::stone_scroll(d, total);
    ::core::ops::ControlFlow::Break(())
}
