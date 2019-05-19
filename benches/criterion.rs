extern crate matchbook;
use matchbook::{Order, OrderBook, Price, Quantity};

extern crate criterion;
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_inserts(c: &mut Criterion) {
  c.bench_function_over_inputs(
    "insert same price orders with book size",
    |b, &n_orders| {
      let mut book = OrderBook::default();
      for _ in 0..n_orders as usize {
        book.insert_ask(Order::new(100.into(), 100.into()));
      }

      b.iter(|| {
        book.insert_ask(Order::new(100.into(), 100.into()));
      });
    },
    (0..5).map(|x| 10usize.pow(x)).collect::<Vec<usize>>(),
  );
}

fn bench_cancels(c: &mut Criterion) {
  c.bench_function_over_inputs(
    "cancel 1 same price order with book size",
    |b, &n_orders| {
      let mut book = OrderBook::default();
      for _ in 0..n_orders - 1 as usize {
        book.insert_ask(Order::new(100.into(), 100.into()));
      }
      let id = book.insert_ask(Order::new(100.into(), 100.into()));

      b.iter(|| book.cancel_ask(id));
    },
    (0..5).map(|x| 10usize.pow(x)).collect::<Vec<usize>>(),
  );
}

fn bench_updates(c: &mut Criterion) {
  // unimplemented!()
}

fn bench_gets(c: &mut Criterion) {
  // unimplemented!()
}

criterion_group!(benches, bench_inserts, bench_cancels, bench_gets, bench_updates);
criterion_main!(benches);
