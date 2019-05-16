//! Order strucuts

use derivative::Derivative;
use derive_more::{Add, AddAssign, From, Into, Sub};
// TODO: ord needs to be reversed for this type so that the *oldest* id takes priority
/// An `Order` id
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, AddAssign, Derivative, From, Into)]
#[derivative(Debug = "transparent")]
pub struct OrderId(pub usize);

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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, Derivative, Default)]
#[derivative(Debug = "transparent")]
pub struct Price(pub u64);

/// A quantity
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, Derivative, Default)]
#[derivative(Debug = "transparent")]
pub struct Quantity(pub u64);

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
}

#[derive(Debug, Clone, Copy)]
pub struct Execution {
  pub ask_id: OrderId,
  pub bid_id: OrderId,
  pub filled: Quantity,
}
