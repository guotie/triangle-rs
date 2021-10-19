# tri-angle Trader

tri-angle trader without trade, just watch pair's price change, print arbtrage chance.


# 中文

binance 三角套利交易，目前trade部分未开源。

## 原理

根据配置文件，计算出quote的套利交易对列表，然后，根据交易对列表，得到 `HashMap<Symbol-Id, Vec<Tri-Angle-Pair>>`， 即交易对与该交易对相关的三角套利组合数组的map。


开启监听binance ws接口，并初始化所有交易对的初始ticker数据。

然后，监听交易对的价格变化，触发对上面map中该交易对对应的三角套利组合的计算，计算出可获利的交易对，打印获利比例， 交易数量等信息。

如果你需要使用代理才能范围binance ws接口，则你需要使用全局vpn来运行，例如 `netch` 是个非常不错的选择。