use crate::extract;
use serde_json;
use std::fs;

pub struct Data {
    pub netincomeloss: Vec<(String, i64)>,       // 순이익
    pub assets: Vec<(String, i64)>,              // 총자산
    pub stockholdersequity: Vec<(String, i64)>,  // 자기자본
    pub revenues: Vec<(String, i64)>,            // 매출
    pub operatingincomeloss: Vec<(String, i64)>, // 영업이익
    pub liabilities: Vec<(String, i64)>,         // 부채
}

fn extract_latest(json: &serde_json::Value, keys: &[&str]) -> Vec<(String, i64)> {
    let mut best_data: Vec<(String, i64)> = vec![];
    let mut max_year = 0;

    for key in keys {
        let arr = match json["facts"]["us-gaap"][key]["units"]["USD"].as_array() {
            Some(a) => a,
            None => continue,
        };

        let mut current_data: Vec<(String, i64)> = arr
            .iter()
            .filter(|item| item["form"].as_str() == Some("10-K"))
            .map(|item| {
                (
                    item["end"].as_str().unwrap_or("").to_string(),
                    item["val"].as_i64().unwrap_or(0),
                )
            })
            .filter(|(date, _)| date.len() >= 4)
            .collect();

        if current_data.is_empty() {
            continue;
        }

        current_data.sort_by(|a, b| b.0.cmp(&a.0));
        current_data.dedup_by(|a, b| a.0[..4] == b.0[..4]);

        if let Some((latest_date, _)) = current_data.first() {
            if let Ok(year) = latest_date[..4].parse::<i32>() {
                if year > max_year {
                    max_year = year;
                    best_data = current_data;
                }
            }
        }
    }

    best_data.into_iter().take(5).collect()
}

impl Data {
    pub fn new(ticker: &str) -> Data {
        let path = extract::facts_file_path(ticker);
        let content = fs::read_to_string(path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let netincomeloss = extract_latest(
            &json,
            &[
                "NetIncomeLoss",
                "NetIncomeLossAvailableToCommonStockholdersBasic",
            ],
        );
        let assets = extract_latest(&json, &["Assets"]);
        let liabilities = extract_latest(&json, &["Liabilities", "LiabilitiesCurrent"]);

        let mut stockholdersequity = extract_latest(
            &json,
            &[
                "StockholdersEquity",
                "StockholdersEquityIncludingPortionAttributableToNoncontrollingInterest",
            ],
        );

        if stockholdersequity.is_empty() && !assets.is_empty() && !liabilities.is_empty() {
            let mut calculated_equity = Vec::new();
            for (date, ast_val) in assets.iter() {
                let ast_yr = &date[..4];
                if let Some((_, liab_val)) = liabilities
                    .iter()
                    .find(|(d, _)| d.len() >= 4 && &d[..4] == ast_yr)
                {
                    calculated_equity.push((date.clone(), ast_val - liab_val));
                }
            }
            stockholdersequity = calculated_equity;
        }

        Data {
            netincomeloss,
            assets,
            stockholdersequity,
            revenues: extract_latest(
                &json,
                &[
                    "Revenues",
                    "RevenueFromContractWithCustomerExcludingAssessedTax",
                    "SalesRevenueNet",
                ],
            ),
            operatingincomeloss: extract_latest(&json, &["OperatingIncomeLoss", "OperatingLoss"]),
            liabilities,
        }
    }
}
