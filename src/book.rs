//! Central limit order book (CLOB)

use crate::types::*;
use if_chain::if_chain;
use std::cmp::Reverse;
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrderBook {
  bids: LimitLevels<Reverse<Price>>,
  asks: LimitLevels<Price>,
}

impl OrderBook {
  pub fn spread(&self) -> Option<Price> {
    match (self.asks.price(), self.bids.price()) {
      (Some(ask), Some(bid)) => {
        if ask > bid {
          Some(ask - bid)
        } else {
          Some(bid - ask)
        }
      }
      _ => None,
    }
  }

  pub fn update_bid(&mut self, id: OrderId, maybe_price: Option<Price>, maybe_quantity: Option<Quantity>) -> bool {
    self
      .bids
      .update(id, maybe_price.map(|x| x.into()), maybe_quantity.map(|x| x.into()))
  }

  pub fn update_ask(&mut self, id: OrderId, maybe_price: Option<Price>, maybe_quantity: Option<Quantity>) -> bool {
    self
      .asks
      .update(id, maybe_price.map(|x| x.into()), maybe_quantity.map(|x| x.into()))
  }

  /// Return the current best asking price
  pub fn ask_market_price(&self) -> Option<Price> {
    self.asks.price().map(|x| x.into())
  }

  /// Return the current best bidding price
  pub fn bid_market_price(&self) -> Option<Price> {
    self.asks.price().map(|x| x.into())
  }

  pub fn ask_limit_level(&self, price: Price) -> Option<Vec<Order>> {
    self.asks.level(price.into())
  }

  pub fn cancel_ask(&mut self, id: OrderId) -> bool {
    self.asks.cancel(id.into())
  }

  pub fn cancel_bid(&mut self, id: OrderId) -> bool {
    self.bids.cancel(id.into())
  }

  pub fn insert_ask(&mut self, order: Order) -> OrderId {
    self.asks.insert(order).into()
  }

  pub fn insert_bid(&mut self, order: Order) -> OrderId {
    self.bids.insert(order).into()
  }

  pub fn get_ask(&self, id: OrderId) -> Option<&Order> {
    self.asks.get(id.into())
  }

  pub fn get_bid(&self, id: OrderId) -> Option<&Order> {
    self.bids.get(id.into())
  }

  pub fn execute_bid(&mut self, id: OrderId) -> Vec<(OrderId, Quantity)> {
    if let Some(order) = self.bids.get_mut(id.into()) {
      self.asks.execute(order).into_iter().collect()
    } else {
      vec![]
    }
  }

  pub fn execute_ask(&mut self, id: OrderId) -> Vec<(OrderId, Quantity)> {
    if let Some(order) = self.asks.get_mut(id) {
      self
        .bids
        .execute(order)
        .into_iter()
        .map(|(id, price)| (id.into(), price.into()))
        .collect()
    } else {
      vec![]
    }
  }

  pub fn first_bid(&self) -> Option<&Order> {
    self.bids.first()
  }

  pub fn first_ask(&self) -> Option<&Order> {
    self.asks.first()
  }
}


#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LimitLevels<P>
where
  P: Ord + From<Price> + Into<Price>,
{
  limit_levels: BTreeMap<P, VecDeque<OrderId>>,
  orders: Vec<Order>,
  // TODO: add id -> limit level index map for fast access and deletion
}

impl<P> LimitLevels<P>
where
  P: Ord + From<Price> + Into<Price> + Clone,
{
  pub fn first(&self) -> Option<&Order> {
    self
      .limit_levels
      .values()
      .next()
      .and_then(|level| level.front())
      .and_then(|&id| self.orders.get::<usize>(id.into()))
  }

  pub fn price(&self) -> Option<Price> {
    self.limit_levels.keys().next().cloned().map(P::into)
  }

  /// Insert an order into the book
  pub fn insert(&mut self, order: Order) -> OrderId {
    assert_eq!(order.is_cancelled, false);
    let id = self.orders.len().into();
    let price = order.price;

    self.orders.push(order);
    self.limit_levels.entry(P::from(price)).or_default().push_back(id);

    id
  }

  pub fn get_mut(&mut self, id: OrderId) -> Option<&mut Order> {
    self.orders.get_mut::<usize>(id.into())
  }

  pub fn update(&mut self, id: OrderId, maybe_price: Option<Price>, maybe_quantity: Option<Quantity>) -> bool {
    if let Some(order) = self.orders.get_mut::<usize>(id.into()) {
      if let Some(price) = maybe_price {
        order.price = price;
      }

      if let Some(quantity) = maybe_quantity {
        order.quantity = quantity;
      }

      true
    } else {
      false
    }
  }


  /// Get an order from the book
  pub fn get(&self, id: OrderId) -> Option<&Order> {
    self.orders.get::<usize>(id.into())
  }

  pub fn execute(&mut self, bid: &mut Order) -> Vec<(OrderId, Quantity)> {
    if bid.remaining() == 0.into() {
      return vec![];
    }

    let mut should_remove_level = false; // FIXME: I don't like using this
    let mut executions = vec![]; // FIXME: I don't like using this
    if let Some(limit_level) = self.limit_levels.get_mut(&P::from(bid.price)) {
      while let Some(id) = limit_level.pop_front() {
        let ask = self.orders.get_mut::<usize>(id.into()).unwrap();
        let to_fill = ask.remaining().min(bid.remaining()); // number of fills are bounded by the least remaining
        bid.filled += to_fill;
        ask.filled += to_fill;

        // push order back to front if it's not filled
        if ask.filled < ask.quantity {
          limit_level.push_front(id);
        } else if limit_level.is_empty() {
          should_remove_level = true;
        }

        executions.push((id, to_fill));

        if bid.filled == bid.quantity {
          break;
        }
      }
    }

    if should_remove_level {
      self.limit_levels.remove(&P::from(bid.price));
    }

    executions
  }

  pub fn cancel(&mut self, id: OrderId) -> bool {
    // helper
    let find_index_of_id = |v: &VecDeque<_>| {
      v.iter()
        .enumerate()
        .find(|(_, &other_id)| id == other_id)
        .map(|(i, _)| i)
    };

    if_chain! {
      if let Some(order) = self.orders.get_mut::<usize>(id.into()); // order exists
      // price level exists
      if let Some(limit_level) = self.limit_levels.get_mut(&P::from(order.price));
      // id is in the limit level
      if let Some(removal_index) = find_index_of_id(limit_level);
      then {
        order.is_cancelled = true;
        limit_level.remove(removal_index);

        // if no other prices at this limit level exist, remove it
        if limit_level.is_empty() {
          self.limit_levels.remove(&order.price.into());
        }

        true
      } else {
        false
      }
    }
  }

  /// Return all orders at a limit
  pub fn level(&self, price: Price) -> Option<Vec<Order>> {
    let get_order_from_id = |ids: &VecDeque<OrderId>| {
      ids
        .iter()
        .map(|&id| self.orders.get::<usize>(id.into()).unwrap())
        .cloned()
        .collect()
    };

    self.limit_levels.get(&price.into()).map(get_order_from_id)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  extern crate rand;
  use rand::{distributions::Distribution, SeedableRng};

  #[test]
  fn execution_works_correctly() {
    let mut book = OrderBook::default();
    let ask0 = Order::new(100.into(), 100.into());
    let ask1 = Order::new(100.into(), 100.into());
    let ask2 = Order::new(100.into(), 100.into());
    let bid0 = Order::new(100.into(), 250.into());
    let bid1 = Order::new(100.into(), 50.into());
    let ask0_id = book.insert_ask(ask0);
    let ask1_id = book.insert_ask(ask1);
    let ask2_id = book.insert_ask(ask2);
    let bid0_id = book.insert_bid(bid0);
    let bid1_id = book.insert_bid(bid1);

    assert_eq!(book.ask_limit_level(100.into()), Some(vec![ask0, ask1, ask2]));
    assert_eq!(book.ask_market_price(), Some(100.into()));
    assert_eq!(
      book.execute_bid(bid0_id),
      vec![(ask0_id, 100.into()), (ask1_id, 100.into()), (ask2_id, 50.into()),]
    );
    assert_eq!(
      book.ask_limit_level(100.into()),
      Some(vec![Order {
        price: 100.into(),
        quantity: 100.into(),
        filled: 50.into(),
        is_cancelled: false,
      }])
    );
    assert_eq!(book.execute_bid(bid1_id), vec![(ask2_id, 50.into())]);
    assert_eq!(book.ask_limit_level(100.into()), None);
  }

  #[test]
  fn market_ask_price_is_lowest_price() {
    let mut book = OrderBook::default();
    let mut rng = rand::rngs::SmallRng::from_seed([0; 16]);
    let normal = rand::distributions::Normal::new(5_000.0, 10.0);
    let orders: Vec<_> = (0..100_000)
      .map(|_| Order::new((normal.sample(&mut rng) as u32).into(), 100.into()))
      .collect();

    orders.iter().for_each(|&x| {
      book.insert_ask(x);
    });

    let lowest = orders.iter().map(|x| x.price.into()).min();

    assert_eq!(lowest, book.ask_market_price());

  }

  #[test]
  fn cancel_operates_correctly() {
    let mut book = OrderBook::default();
    let order0 = Order::new(100.into(), 100.into());
    let order1 = Order::new(100.into(), 50.into());

    let id0 = book.insert_ask(order0);
    let id1 = book.insert_ask(order1);
    assert_eq!(book.ask_limit_level(100.into()), Some(vec![order0, order1]));
    assert!(!book.asks.limit_levels.is_empty());

    assert_eq!(book.get_ask(id1).map(|order| order.is_cancelled), Some(false));
    assert_eq!(book.cancel_ask(id1), true);
    assert_eq!(book.get_ask(id1).map(|order| order.is_cancelled), Some(true));
    assert_eq!(book.ask_limit_level(100.into()), Some(vec![order0]));
    assert!(!book.asks.limit_levels.is_empty());

    assert_eq!(book.get_ask(id0).map(|order| order.is_cancelled), Some(false));
    assert_eq!(book.cancel_ask(id0), true);
    assert_eq!(book.get_ask(id0).map(|order| order.is_cancelled), Some(true));
    assert_eq!(book.ask_limit_level(100.into()), None);
    assert!(book.asks.limit_levels.is_empty());
  }
}

#[cfg(test)]
mod bench {
  extern crate test;

  use super::*;
  use rand::{
    distributions::{Distribution, Normal},
    Rng, SeedableRng,
  };
  use test::{black_box, Bencher};


  #[bench]
  fn insert_1_order_with_1_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    book.insert_ask(Order::new(10.into(), 100.into()));

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.insert_ask(Order::new(10.into(), 100.into()));
    });
  }

  #[bench]
  fn insert_1_order_with_100k_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 1..100_000 {
      book.insert_ask(Order::new(10.into(), 100.into()));
    }

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.insert_ask(Order::new(10.into(), 100.into()));
    });
  }

  #[bench]
  fn clone_with_100k_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 1..100_000 {
      book.insert_ask(Order::new(10.into(), 100.into()));
    }

    b.iter(|| {
      black_box(book.clone());
    });
  }

  #[bench]
  fn execute_1_order_with_100k_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 0..99_999 {
      book.insert_ask(Order::new(100.into(), 100.into()));
    }
    let id = book.insert_ask(Order::new(100.into(), 100.into()));

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.execute_bid(id);
    });
  }

  #[bench]
  fn execute_1_order_with_100_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 0..99 {
      book.insert_ask(Order::new(100.into(), 100.into()));
    }
    let id = book.insert_ask(Order::new(100.into(), 100.into()));

    b.iter(|| {
      let mut book = black_box(book.clone());
      book.execute_bid(id);
    });
  }

  #[bench]
  fn cancel_1_order_with_100k_single_limit_level(b: &mut Bencher) {
    let mut book = OrderBook::default();
    for _ in 0..99_999 {
      book.insert_ask(Order::new(10.into(), 100.into()));
    }
    let cancel_id = book.insert_ask(Order::new(10.into(), 100.into()));

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
      .map(|_| Order::new((normal.sample(&mut rng) as u32).into(), 100.into()))
      .collect();

    b.iter(|| {
      let mut book = black_box(book.clone());
      orders.clone().into_iter().for_each(|o| {
        book.insert_ask(o);
      });
    })
  }
}
