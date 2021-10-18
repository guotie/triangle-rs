use std::collections::HashMap;
use std::fmt;
// use chrono::TimeZone;
use string_join::Join;
// use time;
use chrono::prelude::Local;

use binance::model::BookTickerEvent;
use crate::trading_pair::TradingPair;

pub const TX_FEE: f64 = 0.999;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    SideBuy,
    SideSell,
}

#[derive(Debug, Default, Clone)]
pub struct Ticker {
    pub idx: u32,
    pub ba: [f64; 2], // best ask; 0 is price, 1 is qty
    pub bb: [f64; 2], // best bid
}

impl Ticker {
    pub fn from(idx: u32, depth: &BookTickerEvent) -> Self {
        Ticker {
            idx,
            ba: [depth.best_ask.parse().unwrap(), depth.best_ask_qty.parse().unwrap()],
            bb: [depth.best_bid.parse().unwrap(), depth.best_bid_qty.parse().unwrap()],
        }
    }
}

// 计算三角套利的盈利
#[derive(Debug, Default, Clone)]
pub struct Profit {
    pub rev: bool,  // true: 从第二个发起; false: p1-p2-p3
    pub ratio: f64,
    pub amount: f64,
    pub profit: f64,
    pub ts: u64, // timestamp
    pub tickers: [Ticker; 3]
}

#[derive(Debug, Clone)]
pub struct TriPair {
    pub coin: String,
    pub name: String,
    pub dirs: [Side; 3],
    pub pairs: [u32; 3],
    pub pairs_name: [String; 3],
    profit: Profit,
}

impl fmt::Display for TriPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
      write!(f, "<TriPair>: coin={} pairs: {} {} {}", self.coin, self.pairs_name[0], self.pairs_name[1], self.pairs_name[2])
    }
}

impl TriPair {
    pub fn new(coin: String, pairs: Vec<&TradingPair>) -> TriPair {
        let n_pairs: [u32; 3] = [pairs[0].get_symbol_idx(), pairs[1].get_symbol_idx(), pairs[2].get_symbol_idx()]; //pairs.clone();
        let pairs_name: [String; 3] = [pairs[0].text(), pairs[1].text(), pairs[2].text()];
        let dirs: [Side; 3];
        let name: String;

        if pairs[0].quote() == pairs[2].quote() {
            // ada/usdt ada/btc btc/usdt
            dirs = [Side::SideBuy, Side::SideSell, Side::SideSell];
            // let ss = [pairs[0].base(), pairs[0].quote(), pairs[2].base()];
            // ss.sort();
            name = "-".join([pairs[0].base(), pairs[0].quote(), pairs[2].base()]);
        } else if pairs[0].quote() == pairs[2].base() {
            // ada/btc ada/usdt btc/usdt
            dirs = [Side::SideBuy, Side::SideSell, Side::SideBuy];
            // let ss = vec!;
            // ss.sort();
            name = "-".join([pairs[0].base(), pairs[0].quote(), pairs[2].quote()]);
        } else {
            panic!("invalid pair: {} {} {}", pairs[0].get_symbol(), pairs[1].get_symbol(), pairs[2].get_symbol());
        }

        TriPair {
            coin,
            name,
            dirs,
            pairs: n_pairs,
            pairs_name,
            profit: Profit::default(),
        }
    }

    // 计算三角套利组合的盈利
    pub fn calc_profit(&self, t0: &Ticker, t1: &Ticker, t2: &Ticker, print: bool) -> Profit {
        // let p0 = self.pairs[0];
        // let p1 = self.pairs[1];
        // let p2 = self.pairs[2];
        if t0.ba[0] == 0.0 { // || t0.ba[1] == 0.0 {
            // println!("angle {} {} ask ticker 0 is zero", self.name, self.pairs_name[0]);
            return Profit::default();
        }
        // if t0.bb[0] == 0.0 || t0.bb[1] == 0.0 {
        //     println!("angle {} {} bid ticker 0 is zero", self.name, self.pairs_name[0]);
        //     return Profit::default();
        // }
        if t1.ba[0] == 0.0 { // || t1.ba[1] == 0.0 {
            // println!("angle {} {} ask ticker 0 is zero", self.name, self.pairs_name[1]);
            return Profit::default();
        }
        // if t1.bb[0] == 0.0 || t1.bb[1] == 0.0 {
        //     println!("angle {} {} bid ticker 0 is zero", self.name, self.pairs_name[1]);
        //     return Profit::default();
        // }
        if t2.ba[0] == 0.0 { //|| t2.ba[1] == 0.0 {
            // println!("angle {} {} ask ticker 0 is zero", self.name, self.pairs_name[2]);
            return Profit::default();
        }
        // if t2.bb[0] == 0.0 || t2.bb[1] == 0.0 {
        //     println!("angle {} {} bid ticker 0 is zero", self.name, self.pairs_name[2]);
        //     return Profit::default();
        // }

        let mut vol = t0.ba[1]; // p0.asks.qty;
        if vol > t1.bb[1] {
            vol = t1.bb[1];
        }
        // token/btc token/usdt btc/usdt
        // 1. 买 token 需要的 btc
        let btc = vol * t0.ba[0] / TX_FEE;
        // 2. 卖出 token 得到的 usdt
        let usdt = vol * t1.bb[0] * TX_FEE;
        let end_btc;
        if self.dirs[2] == Side::SideSell {
            end_btc = usdt * t2.bb[0] * TX_FEE;
        } else {
            end_btc = usdt * TX_FEE / t2.ba[0];
        }

        // 第二种情况
        let mut vol2: f64 = t1.ba[1];
        if vol2 > t0.bb[1] {
            vol2 = t0.bb[1];
        }
        // 1. 通过 b2/q2 买入, 需要的 usdt 数量
        let usdt2 = vol2 * t1.ba[0] / TX_FEE;
        // 2. 通过 b1/q1 卖出, 得到的 btc 数量
        let btc2 = vol2 * t0.bb[0] * TX_FEE;
        let end_usdt: f64;
        let profit2: f64;
        let profit2_btc: f64;
        if self.dirs[2] == Side::SideSell {
            // token/q1 token/q2 q2/q1 buy usdt/btc
            end_usdt = btc2 * TX_FEE / t2.ba[0];
            profit2 = end_usdt - usdt2;
            profit2_btc = profit2 * t2.bb[0];
        } else {
            // q1/q2 sell btc/usdt
            end_usdt = btc2 * t2.bb[0] * TX_FEE;
            profit2 = end_usdt - usdt2;
            profit2_btc = profit2 / t2.ba[0];
        }

        let mut profit = end_btc - btc;
        let ratio: f64;
        let rev: bool;
        if profit < profit2_btc {
            profit = profit2;
            vol = vol2;
            ratio = profit / usdt2;
            rev = true;
        } else {
            ratio = profit / btc;
            rev = false;
        }
        if profit > 0.0 && print {
            println!("calc profit: t1: [{} {}]  [{} {}]", t0.ba[0], t0.ba[1], t0.bb[0], t0.bb[1]);
            println!("calc profit: t2: [{} {}]  [{} {}]", t1.ba[0], t1.ba[1], t1.bb[0], t1.bb[1]);
            println!("calc profit: t3: [{} {}]  [{} {}]", t2.ba[0], t2.ba[1], t2.bb[0], t2.bb[1]);
            if self.dirs[2] == Side::SideSell {
                println!("calc profit1: b1: {} q1: {} b2: {} q2: {} b3: {} q3: {}", vol, btc, vol, usdt, end_btc, usdt);
                println!("calc profit2: b1: {} q1: {} b2: {} q2: {} b3: {} q3: {}", vol2, btc2, vol2, usdt2, usdt2, end_usdt);
            } else {
                println!("calc profit1: b1: {} q1: {} b2: {} q2: {} b3: {} q3: {}", vol, btc, vol, usdt, usdt, end_btc);
                println!("calc profit2: b1: {} q1: {} b2: {} q2: {} b3: {} q3: {}", vol2, btc2, vol2, usdt2, usdt2, end_usdt);
            }
        }

        Profit {
            rev,
            ratio,
            amount: vol,
            profit,
            ts:  Local::now().timestamp_millis() as u64, // time::OffsetDateTime::now_utc().unix_timestamp_nanos() as u64, // timestamp
            tickers: [
                t0.clone(),
                t1.clone(),
                t2.clone(),
            ]
        }
    }
}

// 三角套利的 桥 交易对, 例如 BTC/USDT ETH/USDT ETH/BTC
fn get_bridge_pairs<'a>(
        pairs: &'a Vec<TradingPair>,
        base_quotes: &'a Vec<String>
    ) -> HashMap<String, &'a TradingPair> {
    let mut bp: HashMap<String, &TradingPair> = HashMap::new();
    let l = base_quotes.len();
    let mpairs: HashMap<String, &TradingPair> = pairs.iter().map(|x| (x.text(), x)).collect();

    for i in 0..l {
        let t1 = base_quotes[i].clone();
        for j in i+1..l {
            let t2 = base_quotes[j].clone();
            let s1 = t1.clone() + "/" + &t2;
            let s2 = t2 + "/" + &t1;
            if mpairs.get(&s1).is_some() {
                bp.insert(s1.clone(), mpairs.get(&s1).unwrap());
            } else if mpairs.get(&s2).is_some() {
                bp.insert(s2.clone(), mpairs.get(&s2).unwrap());
            }
        }
    }

    bp
}

// 推导出三角套利交易对
// 注意: 所有三角套利组合的 pair 都是引用类型, 即所有三角套利组合共享交易对结构体 TradingPair, 因此, 
// 可以让所有的三角套利组合共享交易对的最新 ticker 数据
pub fn derive_tri_pairs(
        pairs: &Vec<TradingPair>,
        base_quotes: &Vec<String>,
        allow_coins: Option<Vec<String>>,
        exclude_coins: Option<Vec<String>>
    ) -> HashMap<String, Vec<TriPair>> {
    // let pairs_len = pairs.len();
    // let base_pairs: Vec<_> = pairs
    //     .iter()
    //     .filter(|&x| {
    //         base_quotes.contains(&x.quote())
    //         // base_quotes.iter().find(|&ele| ele.eq(&x.quote()) ).is_some()
    //     })
    //     .collect();
    let bridges: HashMap<String, &TradingPair> = get_bridge_pairs(pairs, &base_quotes);
    let mut coin_map: HashMap<String, Vec<&TradingPair>> = HashMap::new();

    // 以 base 为key, 把 base 相同的交易对提取出来, 组成一个数组, 作为 value 放到 coin_map
    for pair in pairs.iter() {
        if coin_map.contains_key(&pair.base()) {
            coin_map.get_mut(&pair.base()).unwrap().push(pair);
        } else {
            coin_map.insert(pair.base().clone().to_string(), vec![pair]);
        }
    }

    println!("bridges: {}", bridges.keys().fold("".to_string(), |mut acc, p| { acc.push_str(p); acc.push_str(" "); acc }));
    // TOKENA -> [[TOKENA/BTC, TOKENA/USDT, BTC/USDT], [TOKENA/ETH, TOKENA/BTC, ETH/BTC]]
    let mut tri_pairs_map: HashMap<String, Vec<TriPair>> = HashMap::new();
    for (coin, coin_pairs) in coin_map {
        if (allow_coins.is_some()) && (vec_has_coin(&allow_coins, &coin) == false) {
            continue
        }
        if (exclude_coins.is_some()) && (vec_has_coin(&exclude_coins, &coin) == true) {
            continue
        }
        if let Some(tri_pair) = find_coin_tri_pairs(coin.as_str(), &coin_pairs, &bridges) {
            tri_pairs_map.insert(coin.clone(), tri_pair);
        }
    }

    for (coin, ps) in &tri_pairs_map {
        // let mut pn = "".to_string();
        // ps.iter().map(|x| { pn.push_str(" "); pn.push_str(&x.name); x.coin}).collect();
        let pn = ps.iter().fold("".to_string(), |mut acc, x| { acc.push_str(" "); acc.push_str(&x.name); acc});
        println!("coin: {} pairs: [{}]", coin, pn);
    }

    tri_pairs_map
}

pub fn to_tri_angle_symbol(
        // base_quotes: &Vec<String>,
        // id_pair: &HashMap<u32, TradingPair>,
        tri_pairs_map: &HashMap<String, Vec<TriPair>>,
    ) -> HashMap<u32, Vec<TriPair>> {
    // todo 单独计算 tri_pairs, 且 tri_pairs 的 TriPair 需要是引用类型
    let mut tri_pairs: HashMap<u32, Vec<TriPair>> = HashMap:: new();
    // let bs_map: HashMap<String, bool> = base_quotes.iter().map(|x| (x.clone(), true)).collect();

    // 提取 交易对 关联的三角套利组, 当该交易对的 ticker 变化时, 触发重新计算所有三角套利组的收益
    for (_, pairs) in tri_pairs_map {
        for tp in pairs {
            let p0 = tp.pairs[0];
            let p1 = tp.pairs[1];
            let p2 = tp.pairs[2];

            // if !is_base_symbol(id_pair.get(&p0).unwrap(), &bs_map) {
                if tri_pairs.contains_key(&p0) {
                    tri_pairs.get_mut(&p0).unwrap().push(tp.clone());
                } else {
                    tri_pairs.insert(p0, vec![tp.clone()]);
                }
            // }
            // if !is_base_symbol(id_pair.get(&p1).unwrap(), &bs_map) {
                if tri_pairs.contains_key(&p1) {
                    tri_pairs.get_mut(&p1).unwrap().push(tp.clone());
                } else {
                    tri_pairs.insert(p1, vec![tp.clone()]);
                }
            // }
            // if !is_base_symbol(id_pair.get(&p2).unwrap(), &bs_map) {
                if tri_pairs.contains_key(&p2) {
                    tri_pairs.get_mut(&p2).unwrap().push(tp.clone());
                } else {
                    tri_pairs.insert(p2, vec![tp.clone()]);
                }
            // }
        }
    }

    tri_pairs
}

// fn is_base_symbol(
//         tp: &TradingPair,
//         bs_map: &HashMap<String, bool>
//     ) -> bool {
//     bs_map.contains_key(&tp.base()) && bs_map.contains_key(&tp.quote())
// }

fn vec_has_coin(coins: &Option<Vec<String>>, coin: &String) -> bool {
    match coins {
        None => false,
        Some(coins) => coins.contains(&coin)
    }
}

// 根据一个币种和该币种的交易对列表, 结合 base symbols, 得到所有的 三角套利对
pub fn find_coin_tri_pairs(
            coin: &str,
            pairs: &Vec<&TradingPair>,
            bs_map: &HashMap<String, &TradingPair>
        ) -> Option<Vec<TriPair>> {
    let pairs_len = pairs.len();
    let mut tri_pairs: Vec<TriPair> = Vec::new();

    // println!("coin: {} pairs: {}", coin, pairs_len);
    if pairs_len < 2 {
        // println!("pairs less than 2, coin: {}", coin);
        return None
    }

    for (i, pair_a) in pairs.into_iter().enumerate() {
        if i == pairs_len {
            break
        }
        for j in (i+1)..pairs_len {
            let pair_b = pairs[j];
            let base_a = pair_a.quote().clone().to_string();
            let base_b = pair_b.quote().clone().to_string();
            let c1 = base_a.clone() + "/" + base_b.as_str();
            let c2 = base_b + "/" + base_a.as_str();
            let cs: &TradingPair;

            if bs_map.contains_key(c1.as_str()) {
                cs = bs_map[c1.as_str()];
            } else if bs_map.contains_key(c2.as_str()) {
                cs = bs_map[c2.as_str()];
            } else {
                // println!("not found symbol {} {} in base map", c1, c2);
                continue
            }
            let tp = TriPair::new(coin.clone().to_string(), vec![*pair_a, pair_b, cs]);
            tri_pairs.push(tp);
        }
    }
    if tri_pairs.len() == 0 {
        return None
    }

    Some(tri_pairs)
}
