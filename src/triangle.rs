use std::collections::HashMap;
use std::sync::mpsc::Receiver;

use chrono::prelude::Local;
use binance::api::*;
use binance::general::*;
use binance::model::*;
use crate::tri_pair::Ticker;
use crate::tri_pair::{Profit, TriPair, derive_tri_pairs, to_tri_angle_symbol, TX_FEE};
use crate::trading_pair::TradingPair;
use crate::config::Configuration;
use crate::ticker_cache::start_best_ticker;

// 包含所有的交易对及三角交易对
// 交易对和三角组合的对应关系, 例如 btc/usdt eth/usdt eth/btc
// eth/btc: [btc/usdt eth/usdt eth/btc]
// 
#[derive(Clone, Debug)]
pub struct TriAngleArb {
    // id_symbol: HashMap<u32, String>, // id 和交易对的对应关系
    symbol_id: HashMap<String, u32>, // 交易对对应的 id
    id_pairs: HashMap<u32, TradingPair>,
    angles: HashMap<u32, Vec<TriPair>>,  // TradingPair 指向 pairs
}

pub fn get_symbol_id(s: &Symbol) -> String {
	format!("{}/{}", s.base_asset.clone(), s.quote_asset.clone())
}

impl TriAngleArb {
    pub fn new(config_path: &str) -> Self {
        let config: Configuration = Configuration::new(config_path);
        let pairs = get_pairs(config.fee.unwrap_or(TX_FEE));
        println!("got pairs: {}", pairs.len());

        let symbol_id: HashMap<String, u32> = pairs.iter().map(|x| (x.text(), x.get_symbol_idx())).collect();
        let base_quotes: Vec<String> = config.base_quotes.clone().unwrap_or(vec!["BTC".to_string(), "USDT".to_string()]);
        // let id_symbol: HashMap<u32, String> = pairs.iter().map(|x| (x.get_symbol_idx(), x.text())).collect();

        let tri_pairs_map = derive_tri_pairs(
            &pairs,
            &base_quotes, 
            None, 
            config.exclude_coins.clone()
        );
        let id_pairs: HashMap<u32, TradingPair> = 
                pairs
                .iter()
                .map(|x| (x.get_symbol_idx(), x.clone()))
                .collect(); // HashMap::new();

        let angles = to_tri_angle_symbol(&tri_pairs_map); // &base_quotes, &id_pairs, &tri_pairs_map);
        println!("tri-angles length: {} {}", tri_pairs_map.len(), angles.len());
        // println!("tri-angles coins: {:?}", tri_pairs_map.keys());
        // println!("tri-pairs ids: {:?}", angles.keys().fold("".to_string(), |mut acct, id| {
        //     let p = id_pairs.get(id).unwrap();
        //     acct.push_str(&p.text());
        //     acct.push_str(" ");
        //     acct
        // }));
        // println!("symbol ids: {:?}", symbol_id);

        TriAngleArb {
            // id_symbol,
            symbol_id,
            id_pairs,
            angles,
        }
    }

    // 计算该 ticker 造成的收益变动
    pub fn on_ticker(&self, angles: &Vec<TriPair>) -> Profit {
        let mut best_profit: Profit = Profit::default();
        let mut profit;

        for tp in angles {
            let p0 = tp.pairs[0];
            let p1 = tp.pairs[1];
            let p2 = tp.pairs[2];
            let t0 = self.id_pairs.get(&p0).unwrap();
            let t1 = self.id_pairs.get(&p1).unwrap();
            let t2 = self.id_pairs.get(&p2).unwrap();
            profit = tp.calc_profit(&t0.tick, &t1.tick, &t2.tick, false);
            if profit.profit > best_profit.profit {
                best_profit = profit
            }
        }

        best_profit
    }

    pub fn start(&mut self) {
        let symbol_id: HashMap<String, u32> = self.id_pairs.iter().map(|(id, tp)| (tp.get_symbol(), *id)).collect();
        let ticker_rx = start_best_ticker(symbol_id);

        println!("ws bookticker subscribed!");
        self.wait_ticker_initialized(&ticker_rx, 10000);
        // println!("all symbol ticker initialized");

        loop {
            if let Ok(tick) = ticker_rx.try_recv() {
                let idx = tick.idx;
                if let Some(mut pair) = self.id_pairs.get_mut(&tick.idx) {
                    pair.tick = tick;
                }
                if let Some(angles) = self.angles.get(&idx) {
                    let profit = self.on_ticker(angles);

                    if profit.ratio > 0.0 {
                        println!("tripair {} profitable: ratio: {} {} {}",
                            self.id_pairs.get(&idx).unwrap().text(), profit.ratio, profit.amount, profit.profit);
                    }
                }
            }
        }
    }

    // 等待所有的 symbol 全部初始化完成
    fn wait_ticker_initialized(&mut self, recv_rx: &Receiver<Ticker>, ms: i64) -> Vec<u32> {
        let mut mt: HashMap<u32, bool> = self.angles.iter().map(|(idx, _)| (*idx, false)).collect();
        let total = mt.len();
        let mut inited: usize = 0;

        println!("total tri pairs: {}", total);
        let start = Local::now().timestamp_millis();
        let mut now: i64;

        loop {
            if let Ok(tick) = recv_rx.try_recv() {
                if let Some(val) = mt.get(&tick.idx) {
                    if *val == false {
                        mt.insert(tick.idx, true); // [&tick.idx] = true;
                        inited = inited + 1;
                        // println!("initialized {}. pair {} {} ticker: asks: [{} {}] bids: [{} {}]",
                        //     inited, self.id_pairs.get(&tick.idx).unwrap().text(), tick.idx,
                        //     tick.ba[0], tick.ba[1], tick.bb[0], tick.bb[1]);
                        if inited >= total {
                            // panic!("initialized");
                            return vec![];
                        }
                    }
                    let mut pair = self.id_pairs.get_mut(&tick.idx).unwrap();
                    pair.tick = tick
                }
                // pair.bids = tick.bb;
            }
            now = Local::now().timestamp_millis();
            if now - start > ms {
                println!("time up, recv {}/{} pairs ticker", inited, total);
                break;
            }
        }

        let uninited: Vec<u32> = mt.iter().filter(|(_, inited)| false == **inited).map(|x| *x.0 ).collect();
        // println!("uninited {}: {:?}", total-inited, uninited);

        // panic!("stop");
        uninited
    }
}

fn get_pairs(fee: f64) -> Vec<TradingPair> {
    let mut pairs: Vec<TradingPair> = Vec::new();
    let general: General = Binance::new(None, None);
    let result = match general.exchange_info() {
      Ok(answer) => answer,
      Err(e) => panic!("Error on getting exchange info: {}", e),
    };
    // symbol 与 id 对应关系
    let mut symbol_id_map: HashMap<String, String> = HashMap::new();
    let mut idx: u32 = 1;
    for symbol in &result.symbols {
      // Checks if symbol is currently trading
      if symbol.status == "TRADING" {
        symbol_id_map.insert(symbol.symbol.clone(), get_symbol_id(symbol));

        let mut step: f64 = 0.0;
        // Get step for this symbol
        for filter in &symbol.filters {
          if let Filters::LotSize {
            min_qty: _,
            max_qty: _,
            step_size,
          } = filter
          {
            step = step_size.parse().unwrap()
          };
        }
        pairs.push(TradingPair::new(
          idx,
          symbol.symbol.to_string(),
          symbol.base_asset.to_string(),
          symbol.quote_asset.to_string(),
          step,
          fee,
        ));
        idx += 1;
      }
    }
    pairs
}