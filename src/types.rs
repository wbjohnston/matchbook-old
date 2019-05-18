//! Order strucuts


use derivative::Derivative;
use derive_more::{Add, AddAssign, From, Into, Sub};
use std::cmp::Reverse;

/// An `Order` id
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, AddAssign, Derivative, From, Into)]
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, Derivative, Default, From, Into)]
#[derivative(Debug = "transparent")]
pub struct Price(u32);

/// A quantity
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, Derivative, Default, From, Into)]
#[derivative(Debug = "transparent")]
pub struct Quantity(u32);

/// An order
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

  pub const fn new_partially_filled(price: Price, quantity: Quantity, filled: Quantity) -> Self {
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
}


/// Side of an order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
  Bid,
  Ask,
}
