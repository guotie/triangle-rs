use std::fmt;
use std::time::SystemTime;

use crate::tri_pair::Ticker;
/*
 *  TradingPair
 */
#[derive(Debug, Clone)]
pub struct TradingPair {
	idx: u32,
    symbol: String,
    symbol_id: String,
    base_asset: String,
    quote_asset: String,
    pub step: f64,
    pub tick: Ticker,
    // pub bids: Bids,
    pub fee: f64,
    timestamp: SystemTime,
}

impl fmt::Display for TradingPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<TradingPair {}/{}>", self.base_asset, self.quote_asset)
    }
}

impl TradingPair {
    // Constructor
    pub fn new(idx: u32,
             symbol: String,
             base_asset: String,
             quote_asset: String,
             step: f64,
             fee: f64
            ) -> TradingPair {
		let symbol_id = format!("{}/{}", base_asset.clone(), quote_asset.clone());
		TradingPair {
			idx,
			symbol,
			symbol_id,
			base_asset,
			quote_asset,
			step,
            tick: Ticker::default(),
			// asks: Asks{ price: 0.0, qty: 0.0, },
			// bids: Bids{ price: 0.0, qty: 0.0, },
            fee,
			timestamp: SystemTime::now(),
		}
    }

    // // Setters
    // pub fn update(&mut self, timestamp: SystemTime, asks: Asks, bids: Bids) {
    //     self.asks = asks;
    //     self.bids = bids;
    //     self.timestamp = timestamp;
    // }

    // pub fn update_best_ticker(&mut self, asks: Asks, bids: Bids) {
    //   	self.asks = asks;
    //   	self.bids = bids;
    // }

    // Getters
    pub fn get_symbol(&self) -> String {
      	self.symbol.to_string()
    }

    pub fn get_symbol_id(&self) -> String {
        self.symbol_id.to_string()
    }
    
    pub fn get_symbol_idx(&self) -> u32 {
        self.idx
    }
    
    pub fn get_step(&self) -> f64 {
        self.step
    }
    
    pub fn quote(&self) -> String {
        self.quote_asset.to_string()
    }

    pub fn base(&self) -> String {
        self.base_asset.to_string()
    }

    pub fn get_base_asset(&self) -> String {
        self.base_asset.to_string()
    }

    pub fn get_quote_asset(&self) -> String {
        self.quote_asset.to_string()
    }

    // Utilities
    pub fn has_asset(&self, asset: String) -> bool {
        (asset == self.quote_asset) || (asset == self.base_asset)
    }

    pub fn get_the_other(&self, asset: String) -> String {
        if asset == self.quote_asset {
            self.base_asset.to_string()
        } else if asset == self.base_asset {
            self.quote_asset.to_string()
        } else {
            "".to_string()
        }
    }

    pub fn text(&self) -> String {
        format!("{}/{}", self.base_asset, self.quote_asset)
    }
}

// Eq overloading
impl PartialEq for TradingPair {
    fn eq(&self, other: &Self) -> bool {
      ((other.quote_asset == self.quote_asset) && (other.base_asset == self.base_asset))
        || ((other.quote_asset == self.base_asset) && (other.base_asset == self.quote_asset))
    }
}

impl Eq for TradingPair {}
