use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use binance::websockets::*;

use crate::tri_pair::{Ticker, TriPair};

#[derive(Debug)]
pub struct TickerCache {
	pairs: HashMap<u32, Vec<TriPair>>,
	symbol_id: HashMap<String, u32>,
}

impl TickerCache {
	pub fn new(pairs: HashMap<String, Vec<TriPair>>) -> TickerCache {
		let mut symbol_id = HashMap::new();
		let mut id: u32 = 1;
		let mut npairs: HashMap<u32, Vec<TriPair>> = HashMap::new();
		
		for (symbol, _) in pairs.iter() {
			symbol_id.insert(symbol.clone(), id);
			npairs.insert(id, pairs.get(symbol).unwrap().to_vec());
			id = id +1;
		}

		TickerCache {
			pairs: npairs,
			symbol_id
		}
	}

	pub fn run(&'static self) {
		thread::spawn(move || {
			self.run_ticker();
		});
	}

	fn run_ticker(&self) {
		// let mut tickers = Arc::new(RwLock::new(HashMap::new()));
		// let queue = start_best_ticker(self.symbol_id.clone());

		// loop {
			// if let Ok(id) = queue.try_recv() {
				// 价格发生变化
				// if let Some(angle) = self.pairs.get(&id) {
				// 	// 触发计算
				// 	for tp in angle {
				// 		// 
				// 		// tp.
				// 	}
				// }
			// }
		// }
	}
}


// 订阅全市场最优价格, 一个线程
pub fn start_best_ticker(
	symbol_id_map: HashMap<String, u32>,
  ) -> Receiver<Ticker> {
	let (in_tx, in_rx): (Sender<Ticker>, Receiver<Ticker>) = mpsc::channel();

	// let thread_queue = queue.clone();
	thread::spawn(move || {
	  loop {
		let keep_running = AtomicBool::new(true);
		let mut web_socket: WebSockets<'_> = WebSockets::new(|event: WebsocketEvent| {
		  if let WebsocketEvent::BookTicker(depth_book_ticker) = event {
				// println!("ws event: {}", depth_book_ticker.symbol);
			  match symbol_id_map.get(depth_book_ticker.symbol.as_str()) {
				None => (),
				Some(id) => {
				    // println!("ws event: {} {}", depth_book_ticker.symbol, id);
					// tickers.write().unwrap().insert(*id, depth_book_ticker);
					let ticker = Ticker::from(*id, &depth_book_ticker);
					in_tx.send(ticker).unwrap();
					()
			  	}
			}
			// .push_back(depth_book_ticker);
			// println!("Adding to queue: {}", queue.read().unwrap().len());
		  };
		  Ok(())
		});

		web_socket.connect("!bookTicker").unwrap(); // check error
		if web_socket.event_loop(&keep_running).is_err() {
			println!("ws error occurs");
		  	thread::sleep(Duration::from_secs(1));
		}
		if web_socket.disconnect().is_err() {}
	  }
	});
  
	in_rx
  }