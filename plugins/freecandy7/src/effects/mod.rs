//! The effect registry. Every effect lives in its own module and is listed once in [`EFFECTS`];
//! the manifest's `effect` choice and the dispatch in `pc_filter` are both built from that list.
//!
//! To add an effect: write `effects/<name>.rs` exporting `pub const EFFECT: Effect`, add
//! `mod <name>;` below and append `<name>::EFFECT` to [`EFFECTS`]. Nothing else changes.

use crate::Job;

mod atmosphere;
pub(crate) mod common;
mod cracked_paint;
mod foil;
mod fur;
mod lightning;
mod materials;
mod neon;
mod paper_cut;
mod texture_materials;
mod textures;
mod treadplate;

/// One effect: its value in the `effect` choice, its manifest `params` entries, and its filter.
#[derive(Clone, Copy)]
pub struct Effect {
    /// The `effect` option that selects it (`[a-z0-9_]`); the dialog shows it capitalised.
    pub name: &'static str,
    /// Its entries for the manifest's `params` object, without the braces (may be empty).
    /// Parameter names are shared by every effect, so each effect should prefix new ones.
    pub params: &'static str,
    /// Filters one band in place; `0` on success, any other value is an error code.
    pub render: fn(&mut Job) -> i32,
}

/// Every effect, in the order the `effect` choice lists them. The first is the default.
pub const EFFECTS: &[Effect] = &[
    lightning::EFFECT,
    textures::EFFECTS[0],
    textures::EFFECTS[1],
    textures::EFFECTS[2],
    textures::EFFECTS[3],
    textures::EFFECTS[4],
    textures::EFFECTS[5],
    textures::EFFECTS[6],
    textures::EFFECTS[7],
    textures::EFFECTS[8],
    textures::EFFECTS[9],
    textures::EFFECTS[10],
    textures::EFFECTS[11],
    materials::EFFECTS[0],
    materials::EFFECTS[1],
    materials::EFFECTS[2],
    materials::EFFECTS[3],
    materials::EFFECTS[4],
    materials::EFFECTS[5],
    materials::EFFECTS[6],
    materials::EFFECTS[7],
    materials::EFFECTS[8],
    materials::EFFECTS[9],
    atmosphere::EFFECTS[0],
    atmosphere::EFFECTS[1],
    atmosphere::EFFECTS[2],
    atmosphere::EFFECTS[3],
    atmosphere::EFFECTS[4],
    atmosphere::EFFECTS[5],
    atmosphere::EFFECTS[6],
    atmosphere::EFFECTS[7],
    atmosphere::EFFECTS[8],
    neon::EFFECT,
    foil::EFFECT,
    cracked_paint::EFFECT,
    paper_cut::EFFECT,
];

/// The effect called `name`.
pub fn find(name: &[u8]) -> Option<&'static Effect> {
    EFFECTS.iter().find(|e| e.name.as_bytes() == name)
}
