use crate::book::OrderBook;
use crate::types::*;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use failure::Fail;
use derive_more::{Display, Add, AddAssign, From, Into};
use derivative::Derivative;

// TODO: do not leak out newtypes for this API

/// An order ID
#[derive(
  Clone,
  Copy,
  Serialize,
  Deserialize,
  PartialEq,
  Eq,
  Hash,
  Display,
  Add,
  AddAssign,
  From,
  Into,
  Derivative
)]
#[derivative(Debug = "transparent")]
pub struct Id(usize);

/// An error
#[derive(Debug, Clone, Copy, Fail, Serialize, Deserialize)]
pub enum Error {
  #[fail(display = "account number '{}' does not exist", id)]
  AccountDoesNotExist {
    id: AccountId
  },
  #[fail(display = "symbol '{}' does not exist", symbol)]
  SymbolDoesNotExist {
    symbol: Symbol
  },
  #[fail(display = "order with id '{}' does not exist", id)]
  IdDoesNotExist {
    id: Id
  },
}

/// A match engine command
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Command {
  account_id: AccountId,
  kind: CommandKind,
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
#[derive(Debug, Clone)]
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
      Self::validate_command_against_account(account, &command.kind)?;
      match command.kind {
        ExecuteOrder(id) => {
          // TODO: rewrite this to be more clear
          if let Some(&(symbol, side, book_id)) = self.id_to_order_path_index.get(&id) {
            if let Some(book) = self.books.get_mut(&symbol) {
              let executions = book.execute(side, book_id)
                .iter()
                .cloned()
                // FIXME: this is no good
                .map(|(id, quantity)| (self.order_path_to_id_index.get(&(symbol, side, id)).cloned().unwrap(), quantity))
                .collect();

              Ok(Success::ExecuteOrder(executions))
            } else {
              Err(Error::SymbolDoesNotExist { symbol })
            }
          } else {
            Err(Error::IdDoesNotExist{ id })
          }
        }
        GetOrder(id) => {
          // TODO: rewrite this to be more clear
          if let Some(&(symbol, side, book_id)) = self.id_to_order_path_index.get(&id) {
            if let Some(book) = self.books.get(&symbol) {
              if let Some(order) = book.get(side, book_id) {
                Ok(Success::GetOrder(*order))
              } else {
                unreachable!("should've kept the mapping straight")
              }
            } else {
              Err(Error::SymbolDoesNotExist { symbol })
            }
          } else {
            Err(Error::IdDoesNotExist{ id })
          }
        },

        PlaceOrder(side, symbol, order) => {
          if let Some(book) = self.books.get_mut(&symbol) {
            let book_id = book.insert(side, order);
            let id = self.next_order_id;
            self.next_order_id += 1.into();
            self.id_to_order_path_index.insert(id, (symbol, side, book_id));
            self.order_path_to_id_index.insert((symbol, side, book_id), id);

            Ok(Success::PlaceOrder(id))
          } else {
            Err(Error::SymbolDoesNotExist { symbol })
          }
        },

        CancelOrder(id) => {
          if let Some(&(symbol, side, book_id)) = self.id_to_order_path_index.get(&id) {
            if let Some(book) = self.books.get_mut(&symbol) {
              Ok(Success::CancelOrder(book.cancel(side, book_id)))
            } else {
              Err(Error::SymbolDoesNotExist { symbol })
            }
          } else {
            Err(Error::IdDoesNotExist{ id })
          }
        },

        GetQuote(symbol, side) => {
          if let Some(book) = self.books.get(&symbol) {
            Ok(Success::GetQuote(book.best_price(side)))
          } else {
            Err(Error::SymbolDoesNotExist{ symbol })
          }
        },

        GetAccount(id) => {
          if let Some(account) = self.accounts.get(&id) {
            Ok(Success::GetAccount(account.clone()))
          } else {
            Err(Error::AccountDoesNotExist{ id })
          }
        }
      }
    } else {
      Err(Error::AccountDoesNotExist{ id: command.account_id})
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
      _ => unimplemented!()
    }
  }
}


#[cfg(test)]
mod test {

}
