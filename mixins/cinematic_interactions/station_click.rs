// AbstractContainerScreen.mouseClicked for a station in the scene: no window, so a click on an
// enchanting rune takes that offer, a click on a beacon's power chooses it, a click on the
// payment lights the beacon once a power is chosen; any other click goes to the slot under the
// cursor (a spot of the station or the panel), with the menu's own click.
fn cinematic_station_click(game: &mut crate::game::Game<'_>, right: bool) -> ::core::ops::ControlFlow<bool> {
    use horizon_client::native_cinematic_interactions as ci;
    let kind = ci::kind();
    if !ci::controls() || !ci::is_surface(kind) {
        return ::core::ops::ControlFlow::Continue(());
    }
    let Some(size) = game.renderer.as_ref().map(|r| [r.config.width, r.config.height]) else {
        return ::core::ops::ControlFlow::Break(true);
    };
    let batch = HudBatch::new(size);
    let (x, y) = game.cursor;
    if !right && kind == ci::Kind::Enchanting
        && let Some(i) = ci::rune_at(x, y, batch.width, batch.height)
    {
        game.click_enchant(i);
        ci::start_burst(kind);
        return ::core::ops::ControlFlow::Break(true);
    }
    if !right && kind == ci::Kind::Beacon {
        if let Some(p) = ci::power_at(x, y, batch.width, batch.height) {
            game.beacon_click(ci::POWERS[p].button);
            return ::core::ops::ControlFlow::Break(true);
        }
        let paid = game.station.as_ref().is_some_and(|s| s.inputs.first().is_some_and(|i| i.is_some()) && s.beacon[0].is_some());
        if paid && game.carried.is_none() && ci::surface_hover(ci::ndc((x, y), batch.width, batch.height)) == Some(0) {
            game.beacon_click(ci::BEACON_CONFIRM);
            ci::start_burst(kind);
            return ::core::ops::ControlFlow::Break(true);
        }
    }
    let Some(slot) = game.slot_at(&batch, x, y) else {
        return ::core::ops::ControlFlow::Break(true);
    };
    // The anvil: the hammer swings first, the result is taken when it hits (EnchantFx's tick).
    if matches!(kind, ci::Kind::Anvil | ci::Kind::Grindstone) && slot == 650 && !right {
        if game.slot_view(650).is_some() && game.carried.is_none() {
            ci::start_forge(game.has_shift_down());
        }
        return ::core::ops::ControlFlow::Break(true);
    }
    let input = if game.has_shift_down() { horizon_client::menu::Input::QuickMove } else { horizon_client::menu::Input::Pickup };
    game.menu_click(Some(slot), i32::from(right), input);
    ::core::ops::ControlFlow::Break(true)
}
