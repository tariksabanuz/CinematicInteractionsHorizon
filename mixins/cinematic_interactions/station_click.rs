// AbstractContainerScreen.mouseClicked for a station in the scene: no window, so a click on an
// enchanting rune takes that offer, a click on a beacon's power chooses it, a click on the
// payment lights the beacon once a power is chosen; any other click goes to the slot under the
// cursor (a spot of the station or the panel), with the menu's own click.
fn cinematic_station_click(game: &mut crate::game::Game<'_>, right: bool) -> ::core::ops::ControlFlow<bool> {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
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
    // A shape round the stonecutter's blade: the recipe is chosen (StonecutterMenu.clickMenuButton,
    // with its sound) and one cut is taken into the hand, or with Shift the whole stack into the
    // inventory; a hand holding something else takes nothing.
    if !right && kind == ci::Kind::Stonecutter {
        let seconds = game.started.elapsed().as_secs_f64();
        let picked = game.station.as_ref().filter(|st| st.inputs.first().is_some_and(|i| i.is_some())).and_then(|st| {
            let range = ci::stone_shown(st.stone.recipes.len());
            let k = ci::stone_at(ci::ndc((x, y), batch.width, batch.height), range.len(), seconds)?;
            let index = range.start + k;
            let made = st.stone.recipes.get(index).and_then(|r| game.stations().recipes.stonecutting.get(*r)).map(|c| c.result.clone());
            Some((index, made))
        });
        if let Some((index, made)) = picked {
            if game.carried.as_ref().is_some_and(|c| made.as_ref().is_none_or(|m| m.item != c.item)) {
                return ::core::ops::ControlFlow::Break(true);
            }
            let mut taken = false;
            let flag = &mut taken;
            game.menu_session(|menu, env, _| {
                *flag = menu.button(env, index as i32);
                Some(*flag)
            });
            if taken {
                game.audio.play("minecraft:ui.stonecutter.select_recipe", None, 1.0, 1.0);
                game.remote_station_button(index as i32);
            }
            let input = if game.has_shift_down() { horizon_client::menu::Input::QuickMove } else { horizon_client::menu::Input::Pickup };
            game.menu_click(Some(650), 0, input);
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
