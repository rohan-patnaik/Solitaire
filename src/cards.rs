use serde::{Deserialize, Serialize};

/// A standard French-suited card. Jokers are not part of the core deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    #[must_use]
    pub const fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }

    #[must_use]
    pub const fn color(self) -> Color {
        self.suit.color()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Self; 4] = [Self::Clubs, Self::Diamonds, Self::Hearts, Self::Spades];

    #[must_use]
    pub const fn color(self) -> Color {
        match self {
            Self::Clubs | Self::Spades => Color::Black,
            Self::Diamonds | Self::Hearts => Color::Red,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Rank {
    Ace = 1,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub const ALL: [Self; 13] = [
        Self::Ace,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
        Self::Seven,
        Self::Eight,
        Self::Nine,
        Self::Ten,
        Self::Jack,
        Self::Queen,
        Self::King,
    ];

    #[must_use]
    pub const fn value(self) -> u8 {
        self as u8
    }

    #[must_use]
    pub const fn descending_from(self, lower: Self) -> bool {
        self.value() == lower.value() + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

#[must_use]
pub fn standard_deck() -> Vec<Card> {
    Suit::ALL
        .into_iter()
        .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| Card::new(suit, rank)))
        .collect()
}

/// Deterministically shuffles a slice using `SplitMix64` and Fisher-Yates.
///
/// The algorithm is deliberately owned here rather than delegated to a random
/// crate so a seed always maps to the same deal across releases and platforms.
pub fn shuffle<T>(items: &mut [T], seed: u64) {
    let mut rng = SplitMix64(seed);
    for upper in (1..items.len()).rev() {
        let index = rng.bounded(upper + 1);
        items.swap(upper, index);
    }
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn bounded(&mut self, upper: usize) -> usize {
        let upper = u64::try_from(upper).expect("slice length fits in u64");
        usize::try_from(self.next() % upper).expect("result fits in usize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn standard_deck_is_unique_and_complete() {
        let deck = standard_deck();
        assert_eq!(deck.len(), 52);
        assert_eq!(deck.iter().copied().collect::<HashSet<_>>().len(), 52);
    }

    #[test]
    fn shuffle_is_stable_for_a_seed() {
        let mut first = standard_deck();
        let mut second = standard_deck();
        shuffle(&mut first, 42);
        shuffle(&mut second, 42);
        assert_eq!(first, second);
        assert_ne!(first, standard_deck());
    }

    #[test]
    fn shuffle_preserves_every_card() {
        let expected = standard_deck().into_iter().collect::<HashSet<_>>();
        let mut shuffled = standard_deck();
        shuffle(&mut shuffled, u64::MAX);
        assert_eq!(shuffled.into_iter().collect::<HashSet<_>>(), expected);
    }
}
