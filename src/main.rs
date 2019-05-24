use clap::{App, Arg};
use core::*;

use failure::Error;

use serde_json::{Deserializer, StreamDeserializer};
use std::io::{BufRead, BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
const DEFAULT_PORT: &'static str = "2556";

fn handle_connection(stream: TcpStream, engine: Arc<Mutex<MatchEngine>>) {
  // let deserializer = Deserializer::from_reader(BufReader::new(stream));
  for command in Deserializer::from_reader(stream).into_iter().filter_map(move |x| {
    dbg!(&x);
    Result::ok(x)
  }) {
    let mut lock = engine.lock().unwrap();
    lock.try_process(command);
    dbg!(&lock);
  }

}

fn main() -> Result<(), Error> {
  let matches = App::new(env!("CARGO_PKG_NAME"))
    .version(env!("CARGO_PKG_VERSION"))
    .author(env!("CARGO_PKG_AUTHORS"))
    .about(env!("CARGO_PKG_DESCRIPTION"))
    .arg(Arg::with_name("port").short("p").long("port").help("port to bind to"))
    .get_matches();

  let port = matches.value_of("port").unwrap_or(DEFAULT_PORT).parse::<usize>()?;
  let mut engine = MatchEngine::default();
  engine.insert_new_symbol(['A', 'D', 'B', 'E'].into());
  println!("created account {}", engine.create_account());
  let engine = Arc::new(Mutex::new(engine));


  let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
  for stream in listener.incoming() {
    let stream = stream?;
    let engine = engine.clone();
    thread::spawn(move || {
      handle_connection(stream, engine.clone());
    });
  }

  Ok(())
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn json_schema() {
    println!(
      "{:?}",
      serde_json::to_string(&Command {
        account_id: 0.into(),
        kind: CommandKind::PlaceOrder(
          Side::Ask,
          ['A', 'D', 'B', 'E'].into(),
          Order::new(25.into(), 100.into())
        )
      })
      .unwrap()
    );
  }
}
