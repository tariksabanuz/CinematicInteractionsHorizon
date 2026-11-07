# Notice of the original work and of adapted material

## Cinematic Interactions (MIT)

Cinematic Interactions by Southware (https://github.com/dcvke/CinematicInteractions, also on
CurseForge) is the work this mod ports to Horizon. Its layouts, camera rigs, texts and behaviour
are taken from that mod's code and rewritten in Rust for Horizon. Its MIT licence, reproduced
unchanged as its terms require:

MIT License

Copyright (c) 2026 Southware

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Just a Hammer model (CC BY 4.0)

The anvil's hammer is an adapted version of the "Just a Hammer" model by RDiamonds, licensed
under CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/). The model's boxes (handle, head,
pommel and collar) were converted to Rust constants in `scene.rs` (`HammerModel`), and its size and
pose were changed to fit the anvil scene. No other part of the model is used.

The port itself is by tariksabanuz and is MIT licensed; see LICENSE.
