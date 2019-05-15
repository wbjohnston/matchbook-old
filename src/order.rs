//! Order strucuts


use derivative::Derivative;
use derive_more::{Add, AddAssign, Sub, SubAssign};
use static_assertions::assert_eq_type;
use std::cmp::Ordering;

/// An `Order` id
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderId(pub usize);

/// An integer price
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, SubAssign, Derivative, Default)]
#[derivative(Debug = "transparent")]
pub struct Price(pub u64);

/// A quantity
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Add, AddAssign, Sub, SubAssign, Derivative, Default)]
#[derivative(Debug = "transparent")]
pub struct Quantity(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidOrder {
  pub price: Price,
  pub quantity: Quantity,
  pub filled: Quantity,
}

impl BidOrder {
  pub const fn new(price: Price, quantity: Quantity) -> Self {
    BidOrder {
      price,
      quantity,
      filled: Quantity(0),
    }
  }
}

impl Order for BidOrder {
  fn price(&self) -> Price {
    self.price
  }

  fn quantity_mut(&mut self) -> &mut Quantity {
    &mut self.quantity
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }

  fn filled(&self) -> Quantity {
    self.filled
  }

  fn filled_mut(&mut self) -> &mut Quantity {
    &mut self.filled
  }
}

impl AsRef<BidOrder> for BidOrder {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl PartialOrd for BidOrder {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.price.partial_cmp(&other.price)
  }
}

impl Ord for BidOrder {
  fn cmp(&self, other: &Self) -> Ordering {
    self.price.cmp(&other.price)
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskOrder {
  pub price: Price,
  pub quantity: Quantity,
  pub filled: Quantity,
}

impl AskOrder {
  pub const fn new(price: Price, quantity: Quantity) -> Self {
    AskOrder {
      price,
      quantity,
      filled: Quantity(0),
    }
  }

}

impl AsRef<AskOrder> for AskOrder {
  fn as_ref(&self) -> &Self {
    self
  }
}

impl Order for AskOrder {
  fn price(&self) -> Price {
    self.price
  }

  fn quantity(&self) -> Quantity {
    self.quantity
  }

  fn quantity_mut(&mut self) -> &mut Quantity {
    &mut self.quantity
  }

  fn filled(&self) -> Quantity {
    self.filled
  }

  fn filled_mut(&mut self) -> &mut Quantity {
    &mut self.filled
  }
}

impl PartialOrd for AskOrder {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.price.partial_cmp(&other.price).map(std::cmp::Ordering::reverse)
  }
}

impl Ord for AskOrder {
  fn cmp(&self, other: &Self) -> Ordering {
    self.price.cmp(&other.price)
  }
}

pub(crate) trait Order: Ord {
  /// Return the order price
  fn price(&self) -> Price;

  /// Return the order quantity
  fn quantity(&self) -> Quantity;

  /// Return mutable reference to the quantity of the order that has been filled
  fn quantity_mut(&mut self) -> &mut Quantity;

  /// Return the quantity of the order that has been filled
  fn filled(&self) -> Quantity;

  /// Return mutable reference to the filled of the order that has been filled
  fn filled_mut(&mut self) -> &mut Quantity;

  fn fill<Ask: Order, Bid: Order>(ask: &Ask, bid: &Bid) -> Quantity {
    // FIXME: Need to static assert that `Ask` is a `AskOrder`
    if Self::is_fillable_by(ask, bid) {
      bid.remaining().min(ask.remaining())
    } else {
      Quantity(0)
    }
  }

  /// Return `true` if the order has been filled
  fn is_filled(&self) -> bool {
    self.filled() >= self.quantity()
  }

  /// Return the remaining unfilled quantity
  fn remaining(&self) -> Quantity {
    self.quantity() - self.filled()
  }

  /// Return `true` if
  fn is_fillable_by<Ask: Order, Bid: Order>(ask: &Ask, bid: &Bid) -> bool {
    !ask.is_filled() && !bid.is_filled() && bid.price() == ask.price()
  }

  fn id(&self) -> OrderId {
    // TODO: implement me
    OrderId(0)
  }
}


#[cfg(test)]
mod test {
  use super::*;
  #[test]
  fn fill_returns_remaining_quantity_when_price_is_equal() {
    let bid = BidOrder {
      price: Price(100),
      quantity: Quantity(100),
      filled: Quantity(20),
    };
    let ask = AskOrder::new(Price(100), Quantity(100));

    assert_eq!(BidOrder::fill(&bid, &ask), Quantity(80));
  }
}
