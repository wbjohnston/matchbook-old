#![feature(test)]
pub mod order;
extern crate bitflags;
extern crate derivative;
extern crate static_assertions;

use order::{AskOrder, BidOrder, Order, OrderId, Quantity};

#[derive(Debug, Clone, Default)]
pub struct OrderBook {
  // TODO: change this datastructure to `std::collections::BTreeMap` for a sorted map
  bids: Vec<BidOrder>,
  asks: Vec<AskOrder>,
}

impl OrderBook {
  pub fn submit_ask(&mut self, ask: AskOrder) -> (OrderId, Vec<Fill>) {
    Self::submit(ask, &mut self.asks, &mut self.bids)
  }

  pub fn submit_bid(&mut self, bid: BidOrder) -> (OrderId, Vec<Fill>) {
    Self::submit(bid, &mut self.bids, &mut self.asks)
  }

  // genericizing this seems like it was a huge waste of time imo, I feel like I
  // just wrote the most overcomplicated code in my life.
  // TODO: figure out if this code is even understandable
  /// Submit
  #[inline(always)]
  fn submit<A: Order + Clone + AsRef<A>, B: Order + Clone + AsRef<B>>(
    order: A,
    into: &mut Vec<A>,
    against: &mut Vec<B>,
  ) -> (OrderId, Vec<Fill>) {
    let id = Self::insert_into(order, into);

    // // match orders
    if let Some(head) = against.first_mut() {
      let fills: Vec<_> = OrderBook::matches_mut(head.clone(), into)
        .map(|order_against| {
          let filled = B::fill(head, order_against);
          *order_against.filled_mut() += filled;
          *head.filled_mut() += filled;

          Fill {
            // FIXME: get ids
            ask_id: OrderId(0),
            bid_id: OrderId(0),
            filled,
          }
        })
        .collect();

      // cull filled orders
      // TODO: replace this with removing keys from a b_tree_map
      *into = into.iter().filter(|x| !x.is_filled()).cloned().collect();
      *against = against.iter().filter(|x| !x.is_filled()).cloned().collect();

      (id, fills)
    } else {
      (id, vec![])
    }
  }

  #[inline]
  fn matches_mut<'f, A: Order + AsRef<A> + 'f, B: Order + AsRef<B>>(
    order: A,
    against: &'f mut Vec<B>,
  ) -> impl Iterator<Item = &'f mut B> {
    against
      .iter_mut()
      .filter(move |other_order| A::is_fillable_by(other_order.as_ref(), &order))
  }

  /// Return immutable reference to asks, ordered by priority
  #[inline]
  pub fn asks(&self) -> &[AskOrder] {
    self.asks.as_slice()
  }

  /// Return immutable reference to bids, ordered by priority
  #[inline]
  pub fn bids(&self) -> &[BidOrder] {
    self.bids.as_slice()
  }

  /// Insert order into a list in sorted order and return the index of insertion
  #[inline]
  fn insert_into<T: Order>(order: T, into: &mut Vec<T>) -> OrderId {
    for (i, other) in into.iter().enumerate() {
      if order > *other {
        into.insert(i, order);
        return OrderId(i);
      }
    }
    OrderId(into.len())
  }
}


#[derive(Debug, Clone, Copy)]
pub struct Fill {
  pub ask_id: OrderId,
  pub bid_id: OrderId,
  pub filled: Quantity,
}

#[cfg(test)]
mod test {
  use super::*;
  use order::Price;

  #[test]
  fn inserts_bids_in_sorted_order() {
    let mut book = OrderBook::default();
    book.submit_bid(BidOrder::new(Price(1), Quantity(10)));
    book.submit_bid(BidOrder::new(Price(3), Quantity(20)));
    book.submit_bid(BidOrder::new(Price(2), Quantity(30)));
    book.submit_bid(BidOrder::new(Price(500), Quantity(40)));
    let is_sorted = book
      .bids()
      .windows(2)
      .map(|window| (&window[0], &window[1]))
      .all(|(a, b)| a > b && a.price >= b.price);

    assert!(is_sorted, "bids are not sorted from highest to lowest price")
  }

  #[test]
  fn inserts_asks_in_sorted_order() {
    let mut book = OrderBook::default();
    book.submit_ask(AskOrder::new(Price(1), Quantity(10)));
    book.submit_ask(AskOrder::new(Price(3), Quantity(20)));
    book.submit_ask(AskOrder::new(Price(2), Quantity(30)));
    book.submit_ask(AskOrder::new(Price(500), Quantity(40)));
    let is_sorted = book
      .asks()
      .windows(2)
      .map(|window| (&window[0], &window[1]))
      .all(|(a, b)| a > b && a.price <= b.price);

    assert!(is_sorted, "asks are not sorted from lowest to highest price")
  }

  #[test]
  fn fills_symmetric_order() {
    let mut book = OrderBook::default();
    let (_, ask_fills) = book.submit_ask(AskOrder::new(Price(100), Quantity(100)));
    let (_, bid_fills) = book.submit_bid(BidOrder::new(Price(100), Quantity(100)));

    assert!(ask_fills.is_empty());
    assert_eq!(bid_fills.len(), 1);
    assert_eq!(bid_fills[0].filled, Quantity(100));
    assert_eq!(book.bids().len(), 0);
    assert_eq!(book.asks().len(), 0);
  }

  #[test]
  fn fills_assymetric_order() {
    let mut book = OrderBook::default();
    let (_, ask_fills) = book.submit_ask(AskOrder::new(Price(100), Quantity(50)));
    let (_, bid_fills) = book.submit_bid(BidOrder::new(Price(100), Quantity(100)));

    assert!(ask_fills.is_empty());
    assert_eq!(bid_fills.len(), 1);
    assert_eq!(bid_fills.first().map(|x| x.filled), Some(Quantity(50)));
    assert_eq!(book.bids().len(), 1);
    assert_eq!(book.asks().len(), 0);

    let (_, ask_fills1) = book.submit_ask(AskOrder::new(Price(100), Quantity(50)));

    dbg!(&book);

    assert_eq!(ask_fills1.len(), 1);
    assert_eq!(ask_fills1.first().map(|x| x.filled), Some(Quantity(50)));
    assert_eq!(book.bids().len(), 0);
    assert_eq!(book.asks().len(), 0);
  }

  #[test]
  fn does_not_fill_orders_with_spread() {
    // ask | bid
    //     | 103
    // 100 |
    //     | 101
    // 102
    let mut book = OrderBook::default();
    let (_, ask_fills0) = book.submit_ask(AskOrder::new(Price(100), Quantity(100)));
    let (_, ask_fills1) = book.submit_ask(AskOrder::new(Price(102), Quantity(100)));
    let (_, bid_fills0) = book.submit_bid(BidOrder::new(Price(101), Quantity(100)));
    let (_, bid_fills1) = book.submit_bid(BidOrder::new(Price(103), Quantity(100)));

    assert!(ask_fills0.is_empty());
    assert!(bid_fills0.is_empty());
    assert!(ask_fills1.is_empty());
    assert!(bid_fills1.is_empty());
  }
}

#[cfg(test)]
mod bench {
  use super::*;

  extern crate lazy_static;
  extern crate test;
  use lazy_static::lazy_static;
  use order::Price;
  use test::Bencher;

  #[bench]
  fn submit_ask_with_empty_book(b: &mut Bencher) {
    let mut book = OrderBook::default();
    let to_submit = AskOrder::new(Price(100), Quantity(100));
    b.iter(|| {
      let mut book = book.clone();
      book.submit_ask(to_submit.clone());
    })
  }

  #[bench]
  fn submit_bid_with_empty_book(b: &mut Bencher) {
    let mut book = OrderBook::default();
    let to_submit = BidOrder::new(Price(100), Quantity(100));
    b.iter(|| {
      let mut book = book.clone();
      book.submit_bid(to_submit.clone());
    })
  }

  #[bench]
  fn submit_bid_with_1000_deep_asks(b: &mut Bencher) {
    let mut book = OrderBook::default();
    let asks = (0..).map(|price| AskOrder::new(Price(price), Quantity(100))).take(1000);
    for ask in asks {
      book.submit_ask(ask);
    }
    let to_submit = BidOrder::new(Price(100), Quantity(100));
    b.iter(|| {
      let mut book = book.clone();
      book.submit_bid(to_submit.clone());
    })
  }

  #[bench]
  fn submit_bid_with_100000_deep_bids(b: &mut Bencher) {
    let mut book = OrderBook::default();
    let bids = (0..)
      .skip(100000)
      .map(|price| BidOrder::new(Price(price), Quantity(100)))
      .take(100000);
    let asks = (0..)
      .map(|price| AskOrder::new(Price(price), Quantity(100)))
      .take(100000);

    for bid in bids {
      book.submit_bid(bid);
    }

    for ask in asks {
      book.submit_ask(ask);
    }

    let to_submit = BidOrder::new(Price(100001), Quantity(100));
    b.iter(|| {
      let mut book = book.clone();
      book.submit_bid(to_submit.clone());
    })
  }
}
