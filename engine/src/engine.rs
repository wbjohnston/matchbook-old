use crate::book::OrderBook;
use crate::types::*;
use derivative::Derivative;
use derive_more::{Add, AddAssign, Display, From, Into};
use failure::Fail;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;


// TODO: do not leak out newtypes for this API

/// An order ID
#[derive(
  Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Display, Add, AddAssign, From, Into, Derivative, Default,
)]
#[derivative(Debug = "transparent")]
pub struct Id(usize);

/// An error
#[derive(Debug, Clone, Copy, Fail, Serialize, Deserialize)]
pub enum Error {
  #[fail(display = "account number '{}' does not exist", id)]
  AccountDoesNotExist { id: AccountId },
  #[fail(display = "symbol '{}' does not exist", symbol)]
  SymbolDoesNotExist { symbol: Symbol },
  #[fail(display = "order with id '{}' does not exist", id)]
  IdDoesNotExist { id: Id },
}

/// A match engine command
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Command {
  pub account_id: AccountId,
  pub kind: CommandKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CommandKind {
  // FIXME: these should take in an account id
  CancelOrder(Id),
  PlaceOrder(Side, Symbol, Order),
  GetOrder(Id),
  ExecuteOrder(Id),
  GetQuote(Symbol, Side),
  GetAccount(AccountId),
}

/// Result of a successful match engine processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Success {
  GetOrder(Order),
  PlaceOrder(Id),
  CancelOrder(bool),
  ExecuteOrder(Vec<(Id, Quantity)>),
  GetQuote(Price),
  GetAccount(Account),
}

/// A match engine user account
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Account {
  pub balance: Price,
  pub orders: Vec<Id>,
  pub portfolio: HashMap<Symbol, Quantity>,
}

type OrderPath = (Symbol, Side, OrderId);

/// A central limit order book matching engine
#[derive(Debug, Clone, Default)]
pub struct MatchEngine {
  books: HashMap<Symbol, OrderBook>,
  // NOTE: since id's are given out sequentially and nothing is ever deleted, this can be a Vec
  id_to_order_path_index: HashMap<Id, OrderPath>,
  order_path_to_id_index: HashMap<OrderPath, Id>,
  accounts: HashMap<AccountId, Account>,
  next_order_id: Id,
  next_account_id: AccountId,
}

impl MatchEngine {
  /// Try to process a command
  pub fn try_process(&mut self, command: Command) -> Result<Success, Error> {
    use CommandKind::*;

    if let Some(account) = self.accounts.get_mut(&command.account_id) {
      // Self::validate_command_against_account(account, &command.kind)?;
      match command.kind {
        ExecuteOrder(id) => {
          let (symbol, side, book_id) = self.try_get_order_path(id)?;
          let book = self.try_get_book_mut(symbol)?;
          let executions = book
            .execute(side, book_id)
            .iter()
            .cloned()
            // FIXME: this is no good
            .map(|(id, quantity)| {
              (
                self.order_path_to_id_index.get(&(symbol, side, id)).cloned().unwrap(),
                quantity,
              )
            })
            .collect();

          Ok(Success::ExecuteOrder(executions))
        }
        GetOrder(id) => {
          let (symbol, side, book_id) = self.try_get_order_path(id)?;
          let book = self.try_get_book_mut(symbol)?;
          Ok(Success::GetOrder(*book.get(side, book_id).unwrap()))
        }

        PlaceOrder(side, symbol, order) => {
          let book = self.try_get_book_mut(symbol)?;
          let book_id = book.insert(side, order);
          let id = self.next_order_id;
          self.next_order_id += 1.into();
          self.id_to_order_path_index.insert(id, (symbol, side, book_id));
          self.order_path_to_id_index.insert((symbol, side, book_id), id);

          Ok(Success::PlaceOrder(id))
        }

        CancelOrder(id) => {
          let (symbol, side, book_id) = self.try_get_order_path(id)?;
          let book = self.try_get_book_mut(symbol)?;
          Ok(Success::CancelOrder(book.cancel(side, book_id)))
        }

        GetQuote(symbol, side) => {
          if let Some(book) = self.books.get(&symbol) {
            Ok(Success::GetQuote(book.best_price(side)))
          } else {
            Err(Error::SymbolDoesNotExist { symbol })
          }
        }

        GetAccount(id) => {
          if let Some(account) = self.accounts.get(&id) {
            Ok(Success::GetAccount(account.clone()))
          } else {
            Err(Error::AccountDoesNotExist { id })
          }
        }
      }
    } else {
      Err(Error::AccountDoesNotExist { id: command.account_id })
    }
  }

  pub fn insert_new_symbol(&mut self, symbol: Symbol) -> bool {
    // TODO: we probably don't want to overwrite the order book
    self.books.insert(symbol, OrderBook::default()).is_none()
  }

  /// Create a new account
  ///
  /// # Returns
  /// the id of the created account
  pub fn create_account(&mut self) -> AccountId {
    let id = self.next_account_id;
    self.next_account_id += 1.into();
    self.accounts.insert(id, Account::default());
    id
  }

  fn validate_command_against_account(_account: &Account, _command: &CommandKind) -> Result<(), Error> {
    match _command {
      _ => unimplemented!(),
    }
  }

  fn try_get_account_mut(&mut self, id: AccountId) -> Result<&mut Account, Error> {
    if let Some(account) = self.accounts.get_mut(&id) {
      Ok(account)
    } else {
      Err(Error::AccountDoesNotExist { id })
    }
  }

  fn try_get_book_mut(&mut self, symbol: Symbol) -> Result<&mut OrderBook, Error> {
    if let Some(book) = self.books.get_mut(&symbol) {
      Ok(book)
    } else {
      Err(Error::SymbolDoesNotExist { symbol })
    }
  }

  fn try_get_order_path(&self, id: Id) -> Result<OrderPath, Error> {
    if let Some(path) = self.id_to_order_path_index.get(&id) {
      Ok(*path)
    } else {
      Err(Error::IdDoesNotExist { id })
    }
  }

  fn try_get_path_from_id(&self) -> OrderPath {
    unimplemented!()
  }
}


#[cfg(test)]
mod test {}
