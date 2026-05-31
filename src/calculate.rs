use crate::parse;

fn match_years(
    base: &[(String, i64)],
    other: &[(String, i64)],
    calc: fn(f64, f64) -> f64,
) -> Vec<(String, f64)> {
    let mut result = Vec::new();
    for (date, base_val) in base {
        if date.len() < 4 {
            continue;
        }
        let yr = &date[..4];
        if let Some((_, other_val)) = other
            .iter()
            .find(|(d, _)| d.len() >= 4 && &d[..4] == yr)
        {
            result.push((yr.to_string(), calc(*base_val as f64, *other_val as f64)));
        }
    }
    result
}

fn roe(net_income: &[(String, i64)], equity: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(net_income, equity, |ni, eq| {
        if eq == 0.0 { 0.0 } else { ni / eq * 100.0 }
    })
}
fn roa(net_income: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(net_income, assets, |ni, ast| {
        if ast == 0.0 { 0.0 } else { ni / ast * 100.0 }
    })
}
fn opm(op_income: &[(String, i64)], revenues: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(op_income, revenues, |oi, rv| {
        if rv == 0.0 { 0.0 } else { oi / rv * 100.0 }
    })
}
fn npm(net_income: &[(String, i64)], revenues: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(net_income, revenues, |ni, rv| {
        if rv == 0.0 { 0.0 } else { ni / rv * 100.0 }
    })
}
fn der(liabilities: &[(String, i64)], equity: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(liabilities, equity, |ll, eq| {
        if eq == 0.0 { 0.0 } else { ll / eq * 100.0 }
    })
}
fn er(equity: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(equity, assets, |eq, ast| {
        if ast == 0.0 { 0.0 } else { eq / ast * 100.0 }
    })
}
fn at(revenues: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(revenues, assets, |rv, ast| {
        if ast == 0.0 { 0.0 } else { rv / ast }
    })
}

fn fmt_val(v: i64) -> String {
    let abs = v.unsigned_abs() as f64;
    if abs >= 1_000_000_000_000.0 {
        format!("{:>11.2}T", v as f64 / 1_000_000_000_000.0)
    } else if abs >= 1_000_000_000.0 {
        format!("{:>11.2}B", v as f64 / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:>11.2}M", v as f64 / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:>11.2}K", v as f64 / 1_000.0)
    } else {
        format!("{:>12}", v)
    }
}

fn fmt_raw(vals: &[(String, i64)], years: &[String]) -> String {
    years
        .iter()
        .map(|y| {
            vals.iter()
                .find(|(d, _)| d.len() >= 4 && &d[..4] == y)
                .map(|(_, v)| fmt_val(*v))
                .unwrap_or_else(|| format!("{:>12}", "N/A"))
        })
        .collect::<Vec<_>>()
        .join("")
}

fn fmt_pct(vals: &[(String, f64)], years: &[String]) -> String {
    years
        .iter()
        .map(|y| {
            vals.iter()
                .find(|(yr, _)| yr == y)
                .map(|(_, v)| format!("{:>11.2}%", v))
                .unwrap_or_else(|| format!("{:>12}", "N/A"))
        })
        .collect::<Vec<_>>()
        .join("")
}

fn fmt_x(vals: &[(String, f64)], years: &[String]) -> String {
    years
        .iter()
        .map(|y| {
            vals.iter()
                .find(|(yr, _)| yr == y)
                .map(|(_, v)| format!("{:>11.2}x", v))
                .unwrap_or_else(|| format!("{:>12}", "N/A"))
        })
        .collect::<Vec<_>>()
        .join("")
}

pub fn print(ticker: &str, data: parse::Data) {
    let roe_val = roe(&data.netincomeloss, &data.stockholdersequity);
    let roa_val = roa(&data.netincomeloss, &data.assets);
    let opm_val = opm(&data.operatingincomeloss, &data.revenues);
    let npm_val = npm(&data.netincomeloss, &data.revenues);
    let der_val = der(&data.liabilities, &data.stockholdersequity);
    let er_val  = er(&data.stockholdersequity, &data.assets);
    let at_val  = at(&data.revenues, &data.assets);

    let mut years: Vec<String> = Vec::new();

    for ds in [
        data.revenues.as_slice(),
        data.netincomeloss.as_slice(),
        data.operatingincomeloss.as_slice(),
        data.assets.as_slice(),
        data.liabilities.as_slice(),
        data.stockholdersequity.as_slice(),
    ] {
        for (date, _) in ds {
            if date.len() >= 4 {
                years.push(date[..4].to_string());
            }
        }
    }

    for ds in [
        roe_val.as_slice(), roa_val.as_slice(), opm_val.as_slice(), npm_val.as_slice(),
        der_val.as_slice(), er_val.as_slice(),  at_val.as_slice(),
    ] {
        for (yr, _) in ds {
            years.push(yr.clone());
        }
    }

    years.sort();
    years.dedup();

    if years.is_empty() {
        println!("=== {} ===", ticker);
        println!("  No valid financial data to display.");
        return;
    }

    let sep    = "-".repeat(15 + years.len() * 12);
    let header: String = years.iter().map(|y| format!("{:>12}", y)).collect();

    println!("\n\t\t\t=== {} Financial Summary ===", ticker);
    println!("{:>15}{}", "", header);
    println!("{}", sep);

    for (label, vals) in [
        ("Revenue:",      data.revenues.as_slice()),
        ("Net Income:",   data.netincomeloss.as_slice()),
        ("Oper. Income:", data.operatingincomeloss.as_slice()),
        ("Assets:",       data.assets.as_slice()),
        ("Liabilities:",  data.liabilities.as_slice()),
        ("Equity:",       data.stockholdersequity.as_slice()),
    ] {
        println!("{:<15}{}", label, fmt_raw(vals, &years));
    }

    println!("{}", sep);

    println!("{:<15}{}", "ROE:", fmt_pct(&roe_val, &years));
    println!("{:<15}{}", "ROA:", fmt_pct(&roa_val, &years));
    println!("{:<15}{}", "OPM:", fmt_pct(&opm_val, &years));
    println!("{:<15}{}", "NPM:", fmt_pct(&npm_val, &years));

    println!("{}", sep);

    println!("{:<15}{}", "DER:", fmt_pct(&der_val, &years));
    println!("{:<15}{}", "ER:",  fmt_pct(&er_val,  &years));
    println!("{:<15}{}", "AT:",  fmt_x(&at_val,    &years));

    println!("{}", sep);
}
