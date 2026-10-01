//! What the unit tests share: the content and the founding household.

use crate::content::Content;
use crate::hero::{Hero, HeroId};
use crate::house::House;

/// The content and the founding heroes, in creation order.
pub fn founded() -> (Content, Vec<Hero>) {
    let content = crate::content::load().expect("the content loads");
    let heroes = House::found(&content, 1)
        .expect("the household founds")
        .heroes;
    (content, heroes)
}

/// The id of the founding hero named `name`.
pub fn id(heroes: &[Hero], name: &str) -> HeroId {
    heroes
        .iter()
        .position(|hero| hero.name == name)
        .expect("a founding hero by that name")
}
