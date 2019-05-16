#![feature(test)]
pub mod types;
extern crate bitflags;
extern crate derivative;

extern crate if_chain;

use derivative::Derivative;
use if_chain::if_chain;
use std::cmp::Reverse;
use std::collections::{BTreeMap, VecDeque};
use types::*;


#[derive(Debug, Clone, Default)]
pub struct OrderBook {
  asks: AskBook,
  bids: BidBook,
}

impl OrderBook {
  pub fn insert_ask(&mut self, ask: Order) -> OrderId {
    self.asks.insert(ask)
  }

  // pub fn insert_bid(&mut self, bid: Order) -> OrderId {
  //   self.bids.insert(bid)
  // }

  pub fn cancel_ask(&mut self, id: OrderId) -> bool {
    self.asks.cancel(id)
  }

  // pub fn cancel_bid(&mut self, id: OrderId) -> bool {
  //   self.bids.cancel(id)
  // }

  pub fn execute_ask(&mut self, id: OrderId) -> Vec<Execution> {
    unimplemented!()
  }

  pub fn get_ask(&self, id: OrderId) -> Option<&Order> {
    self.asks.get(id)
  }

  pub fn ask_market_price(&self) -> Option<Price> {
    self.asks.market_price()
  }

  pub fn ask_limit_level(&self, price: Price) -> Option<Vec<Order>> {
    self.asks.limit_level(price)
  }

  pub fn execute_bid(&mut self, id: OrderId) -> Vec<Execution> {
    unimplemented!()
  }
}

#[derive(Debug, Clone, Default)]
pub struct BidBook {}


#[derive(Debug, Clone, Default)]
struct AskBook {
  limit_levels: BTreeMap<Price, VecDeque<OrderId>>,
  orders: Vec<Order>,
}

impl AskBook {
  /// Insert an order into the book
  pub fn insert(&mut self, order: Order) -> OrderId {
    assert_eq!(order.is_cancelled, false);
    let id = self.orders.len().into();
    let price = order.price;

    self.orders.push(order);
    self.limit_levels.entry(price).or_default().push_back(id);

    id
  }

  /// Get an order from the book
  pub fn get(&self, id: OrderId) -> Option<&Order> {
    self.orders.get::<usize>(id.into())
  }

  pub fn execute(&mut self, bid: &mut Order) -> Vec<Execution> {
    if let Some(limit_level) = self.limit_levels.get_mut(&bid.price) {

      let mut executions = vec![];
      let mut to_remove = vec![0];
      while let Some(id) = limit_level.pop_front() {
        let mut ask = self.orders.get_mut::<usize>(id.into()).unwrap();

      }

      executions
    } else {
      vec![]
    }
  }

  pub fn cancel(&mut self, id: OrderId) -> bool {
    if_chain! {
      if let Some(order) = self.orders.get_mut::<usize>(id.into()); // order exists
      // price level exists
      if let Some(limit_level) = self.limit_levels.get_mut(&order.price);
      // id is in the limit level
      if let Some(removal_index) = limit_level.iter().enumerate().find(|(_, &x)| x == id).map(|(i, _)| i);
      then {
        order.is_cancelled = true;
        limit_level.remove(removal_index);

        // if no other prices at this limit level exist, remove it
        if limit_level.is_empty() {
          self.limit_levels.remove(&order.price);
        }

        true
      } else {
        false
      }
    }
  }

  pub fn market_price(&self) -> Option<Price> {
    self.limit_levels.keys().cloned().next()
  }

  /// Return all orders at a limit
  pub fn limit_level(&self, price: Price) -> Option<Vec<Order>> {
    let get_order_from_id = |ids: &VecDeque<OrderId>| {
      ids
        .iter()
        .map(|&id| self.orders.get::<usize>(id.into()).unwrap())
        .cloned()
        .collect()
    };

    self.limit_levels.get(&price).map(get_order_from_id)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  extern crate rand;
  use rand::{distributions::Distribution, SeedableRng};

  #[test]
  fn market_ask_price_is_lowest_price() {
    let mut book = OrderBook::default();
    let mut rng = rand::rngs::SmallRng::from_seed([0; 16]);
    let normal = rand::distributions::Normal::new(5_000.0, 10.0);
    let orders: Vec<_> = (0..100_000)
      .map(|_| Order::new(Price(normal.sample(&mut rng) as u64), Quantity(100)))
      .collect();

    orders.iter().for_each(|&x| {
      book.insert_ask(x);
    });

    let lowest = orders.iter().map(|x| x.price).min();

    assert_eq!(lowest, book.ask_market_price());

  }

  #[test]
  fn cancel_operates_correctly() {
    let mut book = OrderBook::default();
    let order0 = Order::new(Price(100), Quantity(100));
    let order1 = Order::new(Price(100), Quantity(50));

    let id0 = book.insert_ask(order0);
    let id1 = book.insert_ask(order1);
    assert_eq!(book.ask_limit_level(Price(100)), Some(vec![order0, order1]));
    assert!(!book.asks.limit_levels.is_empty());

    assert_eq!(book.get_ask(id1).map(|order| order.is_cancelled), Some(false));
    assert_eq!(book.cancel_ask(id1), true);
    assert_eq!(book.get_ask(id1).map(|order| order.is_cancelled), Some(true));
    assert_eq!(book.ask_limit_level(Price(100)), Some(vec![order0]));
    assert!(!book.asks.limit_levels.is_empty());

    assert_eq!(book.get_ask(id0).map(|order| order.is_cancelled), Some(false));
    assert_eq!(book.cancel_ask(id0), true);
    assert_eq!(book.get_ask(id0).map(|order| order.is_cancelled), Some(true));
    assert_eq!(book.ask_limit_level(Price(100)), None);
    assert!(book.asks.limit_levels.is_empty());
  }
}

#[cfg(test)]
mod bench {
  extern crate rand;
  extern crate test;
  use super::*;
  use rand::distributions::{Distribution, Normal};
  use rand::{Rng, SeedableRng};
  use test::{black_box, Bencher};

  #[bench]
  fn insert_1_order_with_1_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    book.insert_ask(Order::new(Price(10), Quantity(100)));

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.insert_ask(Order::new(Price(10), Quantity(100)));
    });
  }

  #[bench]
  fn insert_1_order_with_100k_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 1..100_000 {
      book.insert_ask(Order::new(Price(10), Quantity(100)));
    }

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.insert_ask(Order::new(Price(10), Quantity(100)));
    });
  }

  #[bench]
  fn cancel_1_order_with_100k_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 1..99_999 {
      book.insert_ask(Order::new(Price(10), Quantity(100)));
    }
    let cancel_id = book.insert_ask(Order::new(Price(10), Quantity(100)));

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.cancel_ask(cancel_id);
    });
  }

  #[bench]
  fn insert_100k_orders_normal_random_prices(b: &mut Bencher) {
    let book = OrderBook::default();
    let mut rng = rand::rngs::SmallRng::from_seed([0; 16]);
    let normal = Normal::new(5_000.0, 10.0);
    let orders: Vec<_> = (0..100_000)
      .map(|_| Order::new(Price(normal.sample(&mut rng) as u64), Quantity(100)))
      .collect();

    b.iter(|| {
      let mut book = black_box(book.clone());
      orders.clone().into_iter().for_each(|o| {
        book.insert_ask(o);
      });
    })
  }
}
