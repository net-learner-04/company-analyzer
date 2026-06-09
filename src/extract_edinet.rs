use crate::parse;
use quick_xml::{events::Event, Reader};
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    collections::HashMap,
    env, fs,
    io::{Cursor, Read},
    path::PathBuf,
    thread,
    time::Duration,
};
use zip::ZipArchive;

fn ca_dir() -> PathBuf {
    let mut p = PathBuf::from(env::var("HOME").expect("HOME not set"));
    p.push("ca_json");
    p
}

fn ensure_dir() {
    let d = ca_dir();
    if !d.exists() {
        fs::create_dir_all(&d).ok();
    }
}

fn index_path(sec: &str) -> PathBuf {
    let mut p = ca_dir();
    p.push(format!("{}-edinet-index.json", sec));
    p
}

fn facts_path(sec: &str) -> PathBuf {
    let mut p = ca_dir();
    p.push(format!("{}-edinet-facts.json", sec));
    p
}

fn stale(path: &PathBuf) -> bool {
    if !path.exists() {
        return true;
    }
    fs::metadata(path)
        .and_then(|m| m.modified())
        .and_then(|t| {
            t.elapsed()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        })
        .map(|d| d >= Duration::from_secs(86400))
        .unwrap_or(true)
}

fn api_key() -> String {
    env::var("EDINET_API_KEY").unwrap_or_else(|_| {
        eprintln!("[Error] Please set the EDINET_API_KEY environment variable.");
        eprintln!("  Issuance : https://api.edinet-fsa.go.jp/");
        eprintln!("  Settings : export EDINET_API_KEY=<key>");
        std::process::exit(1);
    })
}

fn ts_to_date(ts: u64) -> String {
    let d = (ts / 86400) as i64;
    let z = d + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y   = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp  = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let mon = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr  = if mon <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}", yr, mon, day)
}

fn date_ago(n: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    ts_to_date(now.saturating_sub(n * 86400))
}

fn fetch_day(client: &Client, date: &str, key: &str) -> Vec<Value> {
    let url = format!(
        "https://api.edinet-fsa.go.jp/api/v2/documents.json?date={}&type=2&Subscription-Key={}",
        date, key
    );
    match client.get(&url).timeout(Duration::from_secs(15)).send() {
        Ok(r) if r.status().is_success() => r
            .json::<Value>()
            .ok()
            .and_then(|v| v["results"].as_array().cloned())
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn find_reports(client: &Client, sec: &str, key: &str) -> Vec<(String, String)> {
    let idx = index_path(sec);
    if !stale(&idx) {
        if let Ok(s) = fs::read_to_string(&idx) {
            if let Ok(v) = serde_json::from_str::<Vec<(String, String)>>(&s) {
                if !v.is_empty() {
                    return v;
                }
            }
        }
    }

    let sec5 = format!("{}0", sec);
    let mut found: Vec<(String, String)> = Vec::new();

    eprint!("[EDINET] {} Search Annual Reports", sec);

    for day in 0u64..500 {
        if day % 60 == 59 {
            eprint!(".");
        }

        for doc in fetch_day(client, &date_ago(day), key) {
            if doc["formCode"].as_str() != Some("030000") {
                continue;
            }
            let sc = doc["secCode"].as_str().unwrap_or("");
            if sc != sec5 && sc != sec {
                continue;
            }
            let id  = doc["docID"].as_str().unwrap_or("").to_string();
            let end = doc["periodEnd"].as_str().unwrap_or("").to_string();
            if id.is_empty() || end.len() < 4 {
                continue;
            }
            let yr = &end[..4];
            if !found.iter().any(|(_, e)| e.starts_with(yr)) {
                found.push((id, end));
            }
        }

        if found.len() >= 5 {
            break;
        }
        thread::sleep(Duration::from_millis(60));
    }

    eprintln!(" {}건", found.len());
    found.sort_by(|a, b| b.1.cmp(&a.1));
    found.truncate(5);

    if let Ok(s) = serde_json::to_string(&found) {
        let _ = fs::write(&idx, s);
    }
    found
}

fn download_zip(client: &Client, doc_id: &str, key: &str) -> Option<Vec<u8>> {
    let url = format!(
        "https://api.edinet-fsa.go.jp/api/v2/documents/{}?type=5&Subscription-Key={}",
        doc_id, key
    );
    let r = client.get(&url).timeout(Duration::from_secs(60)).send().ok()?;
    if !r.status().is_success() {
        return None;
    }
    Some(r.bytes().ok()?.to_vec())
}

fn xbrl_from_zip(bytes: &[u8]) -> Option<String> {
    let mut arc = ZipArchive::new(Cursor::new(bytes)).ok()?;

    let mut best_i   = None;
    let mut best_sz  = 0usize;
    let mut best_pri = false;

    for i in 0..arc.len() {
        if let Ok(f) = arc.by_index(i) {
            let name = f.name().to_lowercase();
            let sz   = f.size() as usize;
            if !name.ends_with(".xbrl") {
                continue;
            }
            let pri = name.contains("asr") || name.contains("030000");
            if (pri && !best_pri) || (pri == best_pri && sz > best_sz) {
                best_i   = Some(i);
                best_sz  = sz;
                best_pri = pri;
            }
        }
    }

    let mut f = arc.by_index(best_i?).ok()?;
    let mut raw = Vec::new();
    f.read_to_end(&mut raw).ok()?;
    Some(String::from_utf8_lossy(&raw).into_owned())
}

const REV: &[&str] = &[
    "NetSales", "NetSalesAndRevenues", "Revenue", "Revenues",
    "NetSalesSummaryOfBusinessResults",
];
const OPE: &[&str] = &[
    "OperatingIncome", "OperatingProfit", "OperatingProfitLoss", "OperatingIncomeLoss",
];
const NET: &[&str] = &[
    "ProfitAttributableToOwnersOfParent",
    "ProfitLossAttributableToOwnersOfParent",
    "NetIncome", "NetIncomeLoss", "ProfitLoss",
];
const AST: &[&str] = &["Assets", "TotalAssets"];
const LIA: &[&str] = &["Liabilities", "TotalLiabilities"];
const EQT: &[&str] = &[
    "EquityAttributableToOwnersOfParent", "NetAssets",
    "TotalNetAssets", "StockholdersEquity",
];

fn is_target(n: &str) -> bool {
    [REV, OPE, NET, AST, LIA, EQT].iter().any(|t| t.contains(&n))
}

fn ctx_score(c: &str) -> i32 {
    let c = c.to_ascii_lowercase();
    let mut s = 0i32;
    if c.contains("currentyear") { s += 100; }
    if c.contains("consolidated") { s += 30; }
    if c.contains("prior") || c.contains("previous") { s -= 300; }
    if c.contains("nonconsolidated") || c.contains("individual") { s -= 100; }
    if c.contains("segment") { s -= 50; }
    s
}

fn apply_scale(raw: i64, dec: i32) -> i64 {
    let e = (-dec).clamp(-18i32, 18i32);
    if e > 0 {
        raw.saturating_mul(10i64.pow(e as u32))
    } else if e < 0 {
        raw / 10i64.pow((-e) as u32)
    } else {
        raw
    }
}

fn parse_xbrl(xml: &str) -> HashMap<String, Vec<(String, i64)>> {
    let mut rdr = Reader::from_str(xml);
    rdr.trim_text(true);
    let mut buf  = Vec::new();
    let mut data: HashMap<String, Vec<(String, i64)>> = HashMap::new();

    let mut curr_name: Option<String> = None;
    let mut curr_ctx  = String::new();
    let mut curr_dec  = 0i32;

    loop {
        match rdr.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                if is_target(&local) {
                    let mut ctx = String::new();
                    let mut dec = 0i32;
                    let mut nil = false;
                    for a in e.attributes().filter_map(|a| a.ok()) {
                        let k = String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned();
                        let v = String::from_utf8_lossy(a.value.as_ref()).into_owned();
                        match k.as_str() {
                            "contextRef" => ctx = v,
                            "decimals"   => dec = v.parse().unwrap_or(0),
                            "nil"        => nil = v == "true",
                            _ => {}
                        }
                    }
                    if !nil && !ctx.is_empty() {
                        curr_name = Some(local);
                        curr_ctx  = ctx;
                        curr_dec  = dec;
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if curr_name.is_some() {
                    let text = e.unescape().map(|s| s.trim().to_string()).unwrap_or_default();
                    if !text.is_empty() {
                        if let Ok(raw) = text.parse::<i64>() {
                            let name = curr_name.take().unwrap();
                            data.entry(name)
                                .or_default()
                                .push((curr_ctx.clone(), apply_scale(raw, curr_dec)));
                        } else {
                            curr_name = None;
                        }
                    }
                }
            }
            Ok(Event::End(_)) => {
                curr_name = None;
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    data
}

fn pick(data: &HashMap<String, Vec<(String, i64)>>, tags: &[&str]) -> Option<i64> {
    tags.iter()
        .filter_map(|t| data.get(*t))
        .flat_map(|v| v.iter())
        .max_by_key(|(ctx, _)| ctx_score(ctx))
        .map(|(_, v)| *v)
}

fn xbrl_to_row(xml: &str) -> Option<[i64; 6]> {
    let d = parse_xbrl(xml);
    Some([
        pick(&d, REV)?,
        pick(&d, OPE).unwrap_or(0),
        pick(&d, NET).unwrap_or(0),
        pick(&d, AST).unwrap_or(0),
        pick(&d, LIA).unwrap_or(0),
        pick(&d, EQT).unwrap_or(0),
    ])
}

fn save_cache(data: &parse::Data, path: &PathBuf) {
    let v = serde_json::json!({
        "revenues":            data.revenues,
        "operatingincomeloss": data.operatingincomeloss,
        "netincomeloss":       data.netincomeloss,
        "assets":              data.assets,
        "liabilities":         data.liabilities,
        "stockholdersequity":  data.stockholdersequity,
    });
    if let Ok(s) = serde_json::to_string(&v) {
        let _ = fs::write(path, s);
    }
}

fn load_cache(path: &PathBuf) -> Option<parse::Data> {
    let s = fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&s).ok()?;

    fn to_vec(v: &Value) -> Vec<(String, i64)> {
        v.as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                let a = item.as_array()?;
                Some((a[0].as_str()?.to_string(), a[1].as_i64()?))
            })
            .collect()
    }

    Some(parse::Data {
        revenues:            to_vec(&v["revenues"]),
        operatingincomeloss: to_vec(&v["operatingincomeloss"]),
        netincomeloss:       to_vec(&v["netincomeloss"]),
        assets:              to_vec(&v["assets"]),
        liabilities:         to_vec(&v["liabilities"]),
        stockholdersequity:  to_vec(&v["stockholdersequity"]),
    })
}

pub fn get_data(sec_code: &str) -> parse::Data {
    ensure_dir();
    let key    = api_key();
    let client = Client::new();
    let cache  = facts_path(sec_code);

    if !stale(&cache) {
        if let Some(d) = load_cache(&cache) {
            return d;
        }
    }

    let reports = find_reports(&client, sec_code, &key);
    if reports.is_empty() {
        eprintln!("[EDINET] The annual report for stock code {} cannot be found.", sec_code);
        return parse::Data {
            revenues:            vec![],
            operatingincomeloss: vec![],
            netincomeloss:       vec![],
            assets:              vec![],
            liabilities:         vec![],
            stockholdersequity:  vec![],
        };
    }

    let (mut rev, mut ope, mut net, mut ast, mut lia, mut eqt) =
        (vec![], vec![], vec![], vec![], vec![], vec![]);

    for (doc_id, period_end) in &reports {
        eprint!("[EDINET] {} 취득 중...", period_end);

        let zip = match download_zip(&client, doc_id, &key) {
            Some(z) => z,
            None    => { eprintln!(" Download failed"); continue; }
        };
        let xbrl = match xbrl_from_zip(&zip) {
            Some(x) => x,
            None    => { eprintln!(" XBRL extraction failed"); continue; }
        };
        let row = match xbrl_to_row(&xbrl) {
            Some(r) => r,
            None    => { eprintln!(" Parsing Failed"); continue; }
        };
        eprintln!(" Done");

        let [r, o, n, a, l, e] = row;
        let e = if e == 0 && a > 0 { a - l } else { e };

        rev.push((period_end.clone(), r));
        ope.push((period_end.clone(), o));
        net.push((period_end.clone(), n));
        ast.push((period_end.clone(), a));
        lia.push((period_end.clone(), l));
        eqt.push((period_end.clone(), e));

        thread::sleep(Duration::from_millis(300));
    }

    for v in [&mut rev, &mut ope, &mut net, &mut ast, &mut lia, &mut eqt] {
        v.sort_by(|a, b| b.0.cmp(&a.0));
    }

    let data = parse::Data {
        revenues:            rev,
        operatingincomeloss: ope,
        netincomeloss:       net,
        assets:              ast,
        liabilities:         lia,
        stockholdersequity:  eqt,
    };
    save_cache(&data, &cache);
    data
}
