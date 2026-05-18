use reqwest;
use std::path::PathBuf;
use std::{env, fs};

fn directory_path() -> PathBuf {
    let env = env::var("HOME").expect("Failed to find home directory");
    let mut path = PathBuf::from(env);
    path.push("ca_json");
    path
}

fn ticker_file_path() -> PathBuf {
    let mut path = directory_path();
    path.push("company-tickers.json");
    path
}

pub fn facts_file_path(ticker: &str) -> PathBuf {
    let mut path = directory_path();
    path.push(format!("{}-facts.json", ticker));
    path
}

fn directory_check() {
    if !directory_path().exists() {
        fs::create_dir_all(directory_path()).expect("Failed to create directory");
    }
}

pub fn get_company_tickers() {
    directory_check();

    if ticker_file_path().exists() {
        return;
    }

    let client = reqwest::blocking::Client::new();

    let json_file = client
        .get("https://www.sec.gov/files/company_tickers.json")
        .header("User-Agent", "company-analyzer/1.0 pminseo2004@gmail.com")
        .send()
        .unwrap()
        .text()
        .expect("Failed to get json file");

    fs::write(ticker_file_path(), json_file).expect("Failed to write json file");
}

pub fn return_ticker(tkr: &str) -> String {
    let file = fs::read_to_string(ticker_file_path()).unwrap();
    let mut cik_str = String::new();

    let json_file: serde_json::Value = serde_json::from_str(&file)
        .expect("Failed to read the string file after converting it to JSON");

    for (_, value) in json_file.as_object().unwrap() {
        if value["ticker"].as_str() == Some(tkr) {
            let cik = value["cik_str"].as_u64().unwrap();
            cik_str = format!("{:010}", cik);
        }
    }

    cik_str
}

pub fn get_company_facts(ticker: &str, cik_ticker: &str) {
    directory_check();

    if facts_file_path(ticker).exists() {
        return;
    }

    let client = reqwest::blocking::Client::new();

    let json_file = client
        .get(format!(
            "https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json",
            cik_ticker
        ))
        .header("User-Agent", "company-analyzer/1.0 pminseo2004@gmail.com")
        .send()
        .unwrap()
        .text()
        .expect("Failed to get json file");

    fs::write(facts_file_path(ticker), json_file).expect("Failed to write json file");
}
