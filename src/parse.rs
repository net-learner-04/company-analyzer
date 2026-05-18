use crate::extract;
use serde_json;
use std::fs;

pub struct Data {
    pub netincomeloss: Vec<(String, i64)>,        // 순이익
    pub assets: Vec<(String, i64)>,              // 총자산
    pub stockholdersequity: Vec<(String, i64)>,  //자기자본
    pub revenues: Vec<(String, i64)>,            // 매출
    pub operatingincomeloss: Vec<(String, i64)>, // 영업이익
    pub liabilities: Vec<(String, i64)>,         // 부채
}

fn extract_latest(json: &serde_json::Value, key: &str) -> Vec<(String, i64)> {
    let arr = match json["facts"]["us-gaap"][key]["units"]["USD"].as_array() {
        Some(a) => a,
        None => return vec![],
    };

    let mut data: Vec<(String, i64)> = arr.iter()
        .filter(|item| item["form"].as_str() == Some("10-K") && item["fp"].as_str() == Some("FY"))
        .map(|item| (item["end"].as_str().unwrap_or("").to_string(),
                item["val"].as_i64().unwrap_or(0)))
        .collect();

    data.sort_by(|a, b| a.0.cmp(&b.0));

    data.dedup_by(|a, b| a.0 == b.0);

    data.into_iter().rev().take(5).collect()
}

impl Data {
    pub fn new(ticker: &str) -> Data {
        let path = extract::facts_file_path(ticker);
        let content = fs::read_to_string(path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        let info = Data {
            netincomeloss: extract_latest(&json, "NetIncomeLoss"),
            assets: extract_latest(&json, "Assets"),
            stockholdersequity: extract_latest(&json, "StockholdersEquity"),
            revenues: {
                let rv = extract_latest(&json, "Revenues");
                if rv.is_empty() {
                    extract_latest(&json, "RevenueFromContractWithCustomerExcludingAssessedTax")
                } else {
                    rv
                }
            },
            operatingincomeloss: extract_latest(&json, "OperatingIncomeLoss"),
            liabilities: extract_latest(&json, "Liabilities"),
        };

        info
    }
}
