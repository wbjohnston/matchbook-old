//! Order structs

use derivative::Derivative;
use derive_more::{Add, AddAssign, From, Into, Sub, Display};
use serde_derive::{Deserialize, Serialize};
use std::cmp::Reverse;


/// A product symbol
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Derivative, From, Into)]
#[derivative(Debug = "transparent")]
pub struct Symbol([char; 4]);

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      write!(f, "{}{}{}{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// Side of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, Hash)]
pub enum Side {
  Bid,
  Ask,
}

/// An account ID
#[derive(Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize, Display, Add, AddAssign, Derivative, From, Into, Default)]
#[derivative(Debug = "transparent")]
pub struct AccountId(usize);

/// An `Order` id local to the book
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, AddAssign, Derivative, From, Into, Serialize, Deserialize, Display)]
#[derivative(Debug = "transparent")]
pub struct OrderId(usize);

impl From<Price> for Reverse<Price> {
  fn from(value: Price) -> Self {
    Reverse(value)
  }
}

impl Into<Price> for Reverse<Price> {
  fn into(self) -> Price {
    self.0
  }
}

impl Ord for OrderId {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.0.cmp(&other.0)
  }
}

impl PartialOrd for OrderId {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

/// An integer price
#[derive(
  Clone,
  Copy,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Add,
  AddAssign,
  Sub,
  Derivative,
  Default,
  From,
  Into,
  Serialize,
  Deserialize,
  Display,
)]
#[derivative(Debug = "transparent")]
pub struct Price(u32);

/// A quantity
#[derive(
  Clone,
  Copy,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Add,
  AddAssign,
  Sub,
  Derivative,
  Default,
  From,
  Into,
  Serialize,
  Deserialize,
  Display,
)]
#[derivative(Debug = "transparent")]
pub struct Quantity(u32);

/// An order
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Order {
  pub price: Price,
  pub quantity: Quantity,
  pub filled: Quantity,
  pub is_cancelled: bool,
}

impl Order {
  pub const fn new(price: Price, quantity: Quantity) -> Self {
    Self {
      price,
      quantity,
      filled: Quantity(0),
      is_cancelled: false,
    }
  }

  pub fn new_partially_filled(price: Price, quantity: Quantity, filled: Quantity) -> Self {
    assert!(quantity > filled);
    Self {
      price,
      quantity,
      filled,
      is_cancelled: false,
    }
  }

  pub fn remaining(&self) -> Quantity {
    self.quantity - self.filled
  }

  pub fn is_filled(&self) -> bool {
    self.filled >= self.quantity
  }
}
