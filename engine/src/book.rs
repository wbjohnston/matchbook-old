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
  /// Return the current spread
  pub fn spread(&self) -> Price {
    let ask = self.asks.best_price();
    let bid = self.bids.best_price();

    if ask > bid {
      ask - bid
    } else {
      bid - ask
    }
  }

  /// Update an order
  ///
  /// TODO: is this a real thing that can be done?
  pub fn update(
    &mut self,
    side: Side,
    id: OrderId,
    maybe_price: Option<Price>,
    maybe_quantity: Option<Quantity>,
  ) -> bool {
    use Side::*;
    match side {
      Bid => self.bids.update(id, maybe_price, maybe_quantity),
      Ask => self.asks.update(id, maybe_price, maybe_quantity),
    }
  }

  /// Get the best price for the given side
  pub fn best_price(&self, side: Side) -> Price {
    use Side::*;
    match side {
      Ask => self.asks.best_price(),
      Bid => self.bids.best_price(),
    }
  }

  /// Cancel an order
  pub fn cancel(&mut self, side: Side, id: OrderId) -> bool {
    use Side::*;
    match side {
      Bid => self.bids.cancel(id),
      Ask => self.asks.cancel(id),
    }
  }

  /// Insert an order
  pub fn insert(&mut self, side: Side, order: Order) -> OrderId {
    use Side::*;
    match side {
      Ask => self.asks.insert(order),
      Bid => self.bids.insert(order),
    }
  }

  /// Get an order
  pub fn get(&self, side: Side, id: OrderId) -> Option<&Order> {
    use Side::*;
    match side {
      Ask => self.asks.get(id),
      Bid => self.bids.get(id),
    }
  }

  /// Execute an order
  pub fn execute(&mut self, side: Side, id: OrderId) -> (bool, Vec<(OrderId, Quantity, bool)>) {
    use Side::*;
    match side {
      Bid => {
        if let Some(order) = self.bids.get_mut(id) {
          self.asks.execute(order)
        } else {
          unimplemented!()
        }
      }
      Ask => {
        if let Some(order) = self.asks.get_mut(id) {
          self.bids.execute(order)
        } else {
          unimplemented!()
        }
      }
    }
  }

  pub fn level(&self, side: Side, price: Price) -> Option<Vec<OrderId>> {
    use Side::*;
    match side {
      Bid => self.bids.level(price),
      Ask => self.asks.level(price),
    }
  }

  pub fn first(&self) -> Option<(Side, OrderId)> {
    use Side::*;
    match (self.asks.first(), self.bids.first()) {
      (Some(ask), Some(bid)) if ask < bid => Some((Ask, ask)),
      (Some(_), Some(bid)) => Some((Bid, bid)),
      (Some(ask), None) => Some((Ask, ask)),
      (None, Some(bid)) => Some((Bid, bid)),
      _ => None,
    }
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
  pub fn first(&self) -> Option<OrderId> {
    self
      .limit_levels
      .values()
      .next()
      .and_then(|x| VecDeque::front(x).map(|x| *x))
  }

  pub fn remove_from_level(&mut self, id: OrderId) -> bool {
    unimplemented!()
  }

  pub fn best_price(&self) -> Price {
    self
      .limit_levels
      .keys()
      .next()
      .cloned()
      .map(Into::into)
      .unwrap_or_default()
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


  pub fn update(&mut self, id: OrderId, maybe_price: Option<Price>, maybe_quantity: Option<Quantity>) -> bool {
    if let Some(order) = self.orders.get_mut::<usize>(id.into()) {
      if let Some(price) = maybe_price {
        // TODO: this needs to update the index...
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

  pub fn get_mut(&mut self, id: OrderId) -> Option<&mut Order> {
    self.orders.get_mut::<usize>(id.into())
  }

  /// Get an order from the book
  pub fn get(&self, id: OrderId) -> Option<&Order> {
    self.orders.get::<usize>(id.into())
  }

  pub fn execute(&mut self, order: &mut Order) -> (bool, Vec<(OrderId, Quantity, bool)>) {
    if order.remaining() == 0.into() {
      return (true, vec![]);
    }

    let mut should_remove_level = false; // FIXME: I don't like using this
    let mut executions = vec![]; // FIXME: I don't like using this
    if let Some(limit_level) = self.limit_levels.get_mut(&P::from(order.price)) {
      while let Some(id) = limit_level.pop_front() {
        let against = self.orders.get_mut::<usize>(id.into()).unwrap();
        let to_fill = against.remaining().min(order.remaining()); // number of fills are bounded by the least remaining
        order.filled += to_fill;
        against.filled += to_fill;

        // push order back to front if it's not filled
        if against.is_filled() {
          limit_level.push_front(id);
        } else if limit_level.is_empty() {
          should_remove_level = true;
        }

        executions.push((id, to_fill, against.is_filled()));

        if order.filled == order.quantity {
          break;
        }
      }
    }

    if should_remove_level {
      self.limit_levels.remove(&order.price.into());
    }

    (order.is_filled(), executions)
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

  /// Return all orders id at a limit
  pub fn level(&self, price: Price) -> Option<Vec<OrderId>> {
    self
      .limit_levels
      .get(&price.into())
      .map(|level| level.iter().cloned().collect())
  }
}

// #[cfg(test)]
// mod test {
//   use super::*;
//   extern crate rand;
//   use rand::{distributions::Distribution, SeedableRng};

//   #[test]
//   fn execution_works_correctly() {
//     let mut book = OrderBook::default();
//     let ask0 = Order::new(100.into(), 100.into());
//     let ask1 = Order::new(100.into(), 100.into());
//     let ask2 = Order::new(100.into(), 100.into());
//     let bid0 = Order::new(100.into(), 250.into());
//     let bid1 = Order::new(100.into(), 50.into());
//     let ask0_id = book.insert_ask(ask0);
//     let ask1_id = book.insert_ask(ask1);
//     let ask2_id = book.insert_ask(ask2);
//     let bid0_id = book.insert_bid(bid0);
//     let bid1_id = book.insert_bid(bid1);

//     assert_eq!(book.ask_limit_level(100.into()), Some(vec![ask0, ask1, ask2]));
//     assert_eq!(book.ask_market_price(), Some(100.into()));
//     assert_eq!(
//       book.execute_bid(bid0_id),
//       vec![(ask0_id, 100.into()), (ask1_id, 100.into()), (ask2_id, 50.into()),]
//     );
//     assert_eq!(
//       book.ask_limit_level(100.into()),
//       Some(vec![Order {
//         price: 100.into(),
//         quantity: 100.into(),
//         filled: 50.into(),
//         is_cancelled: false,
//       }])
//     );
//     assert_eq!(book.execute_bid(bid1_id), vec![(ask2_id, 50.into())]);
//     assert_eq!(book.ask_limit_level(100.into()), None);
//   }

//   #[test]
//   fn market_ask_price_is_lowest_price() {
//     let mut book = OrderBook::default();
//     let mut rng = rand::rngs::SmallRng::from_seed([0; 16]);
//     let normal = rand::distributions::Normal::new(5_000.0, 10.0);
//     let orders: Vec<_> = (0..100_000)
//       .map(|_| Order::new((normal.sample(&mut rng) as u32).into(), 100.into()))
//       .collect();

//     orders.iter().for_each(|&x| {
//       book.insert_ask(x);
//     });

//     let lowest = orders.iter().map(|x| x.price.into()).min();

//     assert_eq!(lowest, book.ask_market_price());

//   }

//   #[test]
//   fn cancel_operates_correctly() {
//     let mut book = OrderBook::default();
//     let order0 = Order::new(100.into(), 100.into());
//     let order1 = Order::new(100.into(), 50.into());

//     let id0 = book.insert_ask(order0);
//     let id1 = book.insert_ask(order1);
//     assert_eq!(book.ask_limit_level(100.into()), Some(vec![order0, order1]));
//     assert!(!book.asks.limit_levels.is_empty());

//     assert_eq!(book.get_ask(id1).map(|order| order.is_cancelled), Some(false));
//     assert_eq!(book.cancel_ask(id1), true);
//     assert_eq!(book.get_ask(id1).map(|order| order.is_cancelled), Some(true));
//     assert_eq!(book.ask_limit_level(100.into()), Some(vec![order0]));
//     assert!(!book.asks.limit_levels.is_empty());

//     assert_eq!(book.get_ask(id0).map(|order| order.is_cancelled), Some(false));
//     assert_eq!(book.cancel_ask(id0), true);
//     assert_eq!(book.get_ask(id0).map(|order| order.is_cancelled), Some(true));
//     assert_eq!(book.ask_limit_level(100.into()), None);
//     assert!(book.asks.limit_levels.is_empty());
//   }
// }

// #[cfg(test)]
// mod bench {
//   extern crate test;

//   use super::*;
//   use rand::{
//     distributions::{Distribution, Normal},
//     Rng, SeedableRng,
//   };
//   use test::{black_box, Bencher};


//   #[bench]
//   fn insert_1_order_with_1_single_limit_level(b: &mut Bencher) {
//     let mut book = OrderBook::default();
//     book.insert_ask(Order::new(10.into(), 100.into()));

//     b.iter(|| {
//       let mut book = black_box(book.clone());
//       book.insert_ask(Order::new(10.into(), 100.into()));
//     });
//   }

//   #[bench]
//   fn insert_1_order_with_100k_single_limit_level(b: &mut Bencher) {
//     let mut book = OrderBook::default();
//     for _ in 1..100_000 {
//       book.insert_ask(Order::new(10.into(), 100.into()));
//     }

//     b.iter(|| {
//       let mut book = black_box(book.clone());
//       book.insert_ask(Order::new(10.into(), 100.into()));
//     });
//   }

//   #[bench]
//   fn clone_with_100k_single_limit_level(b: &mut Bencher) {
//     let mut book = OrderBook::default();
//     for _ in 1..100_000 {
//       book.insert_ask(Order::new(10.into(), 100.into()));
//     }

//     b.iter(|| {
//       black_box(book.clone());
//     });
//   }

//   #[bench]
//   fn execute_1_order_with_100k_single_limit_level(b: &mut Bencher) {
//     let mut book = OrderBook::default();
//     for _ in 0..99_999 {
//       book.insert_ask(Order::new(100.into(), 100.into()));
//     }
//     let id = book.insert_ask(Order::new(100.into(), 100.into()));

//     b.iter(|| {
//       let mut book = black_box(book.clone());
//       book.execute_bid(id);
//     });
//   }

//   #[bench]
//   fn execute_1_order_with_100_single_limit_level(b: &mut Bencher) {
//     let mut book = OrderBook::default();
//     for _ in 0..99 {
//       book.insert_ask(Order::new(100.into(), 100.into()));
//     }
//     let id = book.insert_ask(Order::new(100.into(), 100.into()));

//     b.iter(|| {
//       let mut book = black_box(book.clone());
//       book.execute_bid(id);
//     });
//   }

//   #[bench]
//   fn cancel_1_order_with_100k_single_limit_level(b: &mut Bencher) {
//     let mut book = OrderBook::default();
//     for _ in 0..99_999 {
//       book.insert_ask(Order::new(10.into(), 100.into()));
//     }
//     let cancel_id = book.insert_ask(Order::new(10.into(), 100.into()));

//     b.iter(|| {
//       let mut book = black_box(book.clone());
//       book.cancel_ask(cancel_id);
//     });
//   }

//   #[bench]
//   fn insert_100k_orders_normal_random_prices(b: &mut Bencher) {
//     let book = OrderBook::default();
//     let mut rng = rand::rngs::SmallRng::from_seed([0; 16]);
//     let normal = Normal::new(5_000.0, 10.0);
//     let orders: Vec<_> = (0..100_000)
//       .map(|_| Order::new((normal.sample(&mut rng) as u32).into(), 100.into()))
//       .collect();

//     b.iter(|| {
//       let mut book = black_box(book.clone());
//       orders.clone().into_iter().for_each(|o| {
//         book.insert_ask(o);
//       });
//     })
//   }
// }
