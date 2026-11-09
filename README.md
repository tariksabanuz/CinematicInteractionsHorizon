# Cinematic Interactions for Horizon

A native mod (`.hnmod`) for [Horizon](https://github.com/horizon-rust/horizon), the Rust version of Minecraft: Java Edition 26.3.

Right-click a crafting table and the table becomes a scene instead of a window:

- From up to five blocks away you first walk to the table, with the view's normal bob, and end up standing right in front of it.
- The camera glides from your eyes to a spot over the table and zooms in a little. Black bars slide in while it moves and slide out when it arrives. Leaving, it glides back.
- Both of your arms, with your own skin, rest on the near edge of the table. The cursor picks a cell of the 3x3 grid: a frame shows around it and the nearest hand reaches for it.
- Items on the grid lie on the table top as small blocks or flat items. The crafted item floats over the far edge; click it to take it. A stack you pick up is in your hand.
- Your inventory is a panel at the top. It shows the hotbar and opens to the whole inventory while the cursor is over it. The controls are listed at the bottom.
- Every click works as in the normal screen: left click takes or puts down, dragging shares, right click places one at a time, shift-click moves quickly. Esc, E, W/A/S/D or jump leaves.
- The book button at the left opens the Recipes panel: a search box, a "Have materials" filter and pages of 16 recipes. Clicking a recipe shows its tabletop guide: the ingredients lie faintly on the cells, framed red where you have none. "Lay out" puts them on the table, and "x" clears the choice.
- Hovering a cell tells what lies on it ("On this cell: 3") or which ingredient the guide wants there.
- Sneak + right-click with an empty hand opens the normal crafting screen.
- Reach: two blocks or more above the table, or one block above and not right beside it, a "Too low" message shows and the table does not open.

Other stations work the same way, in the world instead of a window:

- Furnace, smoker and blast furnace: the items lie in the openings; fuel below, things to smelt above.
- Smithing table, cartography table, grindstone, brewing stand, loom: each input has its own spot on the block. The loom's patterns are on a board at the right once a banner and a dye lie there.
- Enchanting table: a faint blue circle marks the lapis spot; lapis lies on it with its count. The item floats upright over the book, turning, and the three offers float over the table as the jar's turning purple rune circles (cost, a clue in the galactic alphabet, a blue star per lapis); touch one to enchant, and a beam of light, a ring and sparks rise from the item.
- Anvil: the item lies on the anvil with its name floating above (and the cost in green), what goes into it lies on top; LMB on the result swings the jar's hammer down on it; the result is yours when it hits, with sparks.
- Beacon: the powers float beside it in rings of their effect's colour, with the pyramid level they need; a ring lies on the core, the payment on the glass, and a click on it lights the beacon with a beam. The idle hands stay out of the view.
- Crafter: its nine windows are the cells on its top. Items stand in them, a click with an empty hand on an empty window shuts it (a lid lies over it) or opens it again, and what it would craft floats over the far edge.
- Stonecutter: the stone lies by the saw and every shape it can be cut into stands round the blade, with "×2" under the ones that give two. LMB on a shape cuts one into your hand, Shift cuts the whole stack into the inventory, the wheel shows more shapes.
- Anvil rename: click the floating name and a "New name" box opens with the name selected; type, then Enter keeps it and Esc closes the box.
- Chiseled bookshelf: click a book to take it, click an empty place with a book to put it in.
- Campfire and soul campfire: looking at one shows "[H] Sit by the fire". H sits you down by it; carried food goes on a free spot of the fire. Esc stands up.
- Next to another station: the cursor on it outlines it with "Name  RMB — move over", and RMB moves the scene over to it.
- A crafting table keeps what you leave on it (in single player and on a world you host): the items lie on its top in the world and are back in the grid when you open it. Breaking the table drops them.

Options > Mods > Cinematic Interactions turns the mod on and off. It also has settings for the cinematic bars, the control hints and the camera speed (Slow, Normal, Fast).

Needs the Performance build in the launcher.

## Credits

A port of [Cinematic Interactions](https://www.curseforge.com/minecraft/mc-mods/cinematic-interactions) by Southware (MIT) for Fabric and NeoForge: its layouts, camera rigs, texts and behaviour are taken from that mod's code (MIT) and rewritten in Rust for Horizon. The anvil's hammer is an adapted "Just a Hammer" model by RDiamonds (CC BY 4.0). It draws with your own game's textures and skin, and the icon is my own.

## Building

`python3 check/build.py --test` type-checks the hooks against stand-ins for the Horizon signatures they use and runs the tests. `python3 tools/package.py` writes `target/cinematic_interactions-0.12.2.hnmod`.

The original work's MIT notice ships in the bundle as `mixins/cinematic_interactions/NOTICE.md`.

MIT licence.
