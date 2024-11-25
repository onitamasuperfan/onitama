use wasm_bindgen::prelude::*;
use serde::Serialize;
use serde_wasm_bindgen;
use crate::models::{Card, CardDirection, CardSet, Point};
use enum_iterator::IntoEnumIterator;

/// A struct for serializing cards with both normal moves and king moves.
#[derive(Serialize)]
pub struct SerializableCard {
    card: Card,
    moves: Vec<Point>,      // Normal moves
    king_moves: Vec<Point>, // King moves
    direction: CardDirection,
}

impl From<&Card> for SerializableCard {
    fn from(card: &Card) -> Self {
        SerializableCard {
            card: *card,
            moves: card.moves(false, false),     // Normal moves
            king_moves: card.moves(true, false), // King moves
            direction: card.direction(),
        }
    }
}

/// A struct for serializing card sets with `SerializableCard`s.
#[derive(Serialize)]
pub struct SerializableCardSet {
    id: CardSet,
    name: String,
    cards: Vec<SerializableCard>, // Uses SerializableCard instead of CardDescription
}

/// Function to list all card sets with serializable cards.
#[wasm_bindgen(js_name = listCardSets)]
pub fn list_card_sets() -> JsValue {
    let card_sets: Vec<SerializableCardSet> = CardSet::into_enum_iter()
        .map(|card_set| SerializableCardSet {
            id: card_set,
            name: card_set.to_string(),
            cards: card_set
                .cards()
                .iter()
                .map(SerializableCard::from)
                .collect(),
        })
        .collect();

    serde_wasm_bindgen::to_value(&card_sets).unwrap()
}
