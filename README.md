# company-analyzer

A command-line tool written in Rust that pulls public financial data from the SEC EDGAR API and prints a 5-year fundamental analysis summary for any US-listed company.

No API keys, no paid data subscriptions — everything comes from free, public SEC filings.

```
			=== AAPL Financial Summary ===
                       2020        2021        2022        2023        2024
---------------------------------------------------------------------------
Revenue:          274.51B     365.82B     394.33B     383.28B     391.04B
Net Income:        57.41B      94.68B      99.80B      96.99B     101.96B
Oper. Income:      66.29B     108.95B     119.44B     114.30B     123.22B
Assets:           323.89B     351.00B     352.76B     352.58B     364.98B
Liabilities:      258.55B     287.91B     302.08B     290.44B     308.03B
Equity:            65.34B      63.09B      50.67B      62.15B      56.95B
---------------------------------------------------------------------------
ROE:               87.87%     150.07%     196.96%     156.08%     179.02%
ROA:               17.73%      26.97%      28.29%      27.51%      27.94%
OPM:               24.15%      29.78%      30.29%      29.82%      31.51%
NPM:               20.91%      25.88%      25.31%      25.31%      26.08%
---------------------------------------------------------------------------
DER:              395.71%     456.36%     596.42%     468.52%     541.22%
ER:                20.17%      17.97%      14.36%      17.63%      15.60%
AT:                 0.85x       1.04x       1.12x       1.09x       1.07x
---------------------------------------------------------------------------
```

## How it works

1. Downloads `company_tickers.json` from SEC EDGAR to resolve a ticker symbol to a CIK (Central Index Key)
2. Fetches the company's full XBRL facts JSON from `data.sec.gov`
3. Parses annual 10-K filings, selects the most relevant GAAP tag for each metric, and deduplicates to one entry per fiscal year
4. Computes 7 financial ratios and prints a formatted table alongside the raw figures

Both JSON files are cached under `~/ca_json/` and refreshed only when older than 24 hours.

## Metrics

**Raw financials** (auto-scaled to T / B / M / K)

| Field | Source tag(s) |
|---|---|
| Revenue | `Revenues`, `RevenueFromContractWithCustomerExcludingAssessedTax`, `SalesRevenueNet` |
| Net Income | `NetIncomeLoss`, `NetIncomeLossAvailableToCommonStockholdersBasic` |
| Operating Income | `OperatingIncomeLoss`, `OperatingLoss` |
| Assets | `Assets` |
| Liabilities | `Liabilities`, `LiabilitiesCurrent` |
| Equity | `StockholdersEquity` (falls back to Assets − Liabilities) |

**Computed ratios**

| Ratio | Formula | What it tells you |
|---|---|---|
| ROE | Net Income / Equity × 100 | Profitability relative to shareholder capital |
| ROA | Net Income / Assets × 100 | How efficiently assets generate profit |
| OPM | Operating Income / Revenue × 100 | Core operating margin |
| NPM | Net Income / Revenue × 100 | Bottom-line margin after all costs |
| DER | Liabilities / Equity × 100 | Leverage; how much debt backs each dollar of equity |
| ER | Equity / Assets × 100 | Capital structure stability |
| AT | Revenue / Assets | Asset turnover efficiency |

## Installation

Requires Rust and Cargo ([rustup.rs](https://rustup.rs)).

```bash
git clone https://github.com/your-username/company-analyzer
cd company-analyzer
cargo build --release
```

The binary will be at `target/release/company-analyzer`.

## Usage

```bash
# Basic usage
./company-analyzer AAPL

# Other examples
./company-analyzer MSFT
./company-analyzer ECL
./company-analyzer NVDA
```

The ticker must match the symbol listed on SEC EDGAR (usually the same as the NYSE/NASDAQ ticker).

## Project structure

```
src/
├── main.rs        # Entry point and argument handling
├── extract.rs     # SEC EDGAR HTTP client and local JSON cache
├── parse.rs       # XBRL JSON parsing and 5-year data extraction
└── calculate.rs   # Ratio computation and formatted output
```

## Dependencies

```toml
[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }
serde_json = "1.0"
```

## Limitations

- **US companies only.** SEC EDGAR covers companies that file with the SEC; foreign-only listed companies are not available.
- **GAAP tags only.** The parser reads `us-gaap` XBRL tags. Companies that file under IFRS or use non-standard tags may return incomplete data for some metrics.
- **Single-tag selection.** When multiple GAAP tags exist for the same concept (e.g. revenue changed tag names with ASC 606 in 2018), the parser picks the tag with the most recent data. Older filings that used a different tag may be excluded.
- **5-year window.** Only the 5 most recent 10-K entries per metric are shown.

## License

MIT
