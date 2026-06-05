use crate::parse;
use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS,
    presets::UTF8_FULL,
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};

fn match_years(
    base: &[(String, i64)],
    other: &[(String, i64)],
    calc: fn(f64, f64) -> f64,
) -> Vec<(String, f64)> {
    let mut result = Vec::new();
    for (date, base_val) in base {
        if date.len() < 4 { continue; }
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
    match_years(net_income, equity, |ni, eq| if eq == 0.0 { 0.0 } else { ni / eq * 100.0 })
}
fn roa(net_income: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(net_income, assets, |ni, ast| if ast == 0.0 { 0.0 } else { ni / ast * 100.0 })
}
fn opm(op_income: &[(String, i64)], revenues: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(op_income, revenues, |oi, rv| if rv == 0.0 { 0.0 } else { oi / rv * 100.0 })
}
fn npm(net_income: &[(String, i64)], revenues: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(net_income, revenues, |ni, rv| if rv == 0.0 { 0.0 } else { ni / rv * 100.0 })
}
fn der(liabilities: &[(String, i64)], equity: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(liabilities, equity, |ll, eq| if eq == 0.0 { 0.0 } else { ll / eq * 100.0 })
}
fn er(equity: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(equity, assets, |eq, ast| if ast == 0.0 { 0.0 } else { eq / ast * 100.0 })
}
fn at(revenues: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(revenues, assets, |rv, ast| if ast == 0.0 { 0.0 } else { rv / ast })
}

fn debt_ratio(liabilities: &[(String, i64)], assets: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(liabilities, assets, |ll, ast| if ast == 0.0 { 0.0 } else { ll / ast * 100.0 })
}

fn equity_multiplier(assets: &[(String, i64)], equity: &[(String, i64)]) -> Vec<(String, f64)> {
    match_years(assets, equity, |ast, eq| if eq == 0.0 { 0.0 } else { ast / eq })
}

fn yoy_growth(vals: &[(String, i64)]) -> Vec<(String, f64)> {
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut by_year: Vec<(String, i64)> = Vec::new();
    for (date, val) in &sorted {
        if date.len() < 4 { continue; }
        let yr = date[..4].to_string();
        if let Some(last) = by_year.last_mut() {
            if last.0 == yr {
                *last = (yr, *val);
                continue;
            }
        }
        by_year.push((yr, *val));
    }

    let mut result = Vec::new();
    for i in 1..by_year.len() {
        let (yr, cur) = &by_year[i];
        let (_, prev) = &by_year[i - 1];
        let growth = if *prev == 0 {
            0.0
        } else {
            (*cur - *prev) as f64 / prev.unsigned_abs() as f64 * 100.0
        };
        result.push((yr.clone(), growth));
    }
    result
}

fn dupont(
    npm_vals: &[(String, f64)],
    at_vals:  &[(String, f64)],
    em_vals:  &[(String, f64)],
) -> Vec<(String, f64)> {
    let mut result = Vec::new();
    for (yr, npm_v) in npm_vals {
        if let Some((_, at_v)) = at_vals.iter().find(|(y, _)| y == yr) {
            if let Some((_, em_v)) = em_vals.iter().find(|(y, _)| y == yr) {
                result.push((yr.clone(), npm_v / 100.0 * at_v * em_v * 100.0));
            }
        }
    }
    result
}

fn fmt_val(v: i64) -> String {
    let s = v.unsigned_abs().to_string();
    let chunked: String = s
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(",");
    if v < 0 { format!("-{}", chunked) } else { chunked }
}

fn raw_cell(vals: &[(String, i64)], year: &str) -> Cell {
    let found = vals.iter().find(|(d, _)| d.len() >= 4 && &d[..4] == year);
    let text = found
        .map(|(_, v)| fmt_val(*v))
        .unwrap_or_else(|| "N/A".to_string());
    let cell = Cell::new(text).set_alignment(CellAlignment::Right);
    match found {
        Some((_, v)) if *v < 0 => cell.fg(Color::Red),
        _ => cell,
    }
}

fn pct_cell(vals: &[(String, f64)], year: &str) -> Cell {
    let maybe = vals.iter().find(|(yr, _)| yr == year);
    let text = maybe
        .map(|(_, v)| format!("{:.2}%", v))
        .unwrap_or_else(|| "N/A".to_string());
    let cell = Cell::new(text).set_alignment(CellAlignment::Right);
    match maybe {
        Some((_, v)) if *v < 0.0   => cell.fg(Color::Red),
        Some((_, v)) if *v >= 15.0 => cell.fg(Color::Green),
        _ => cell,
    }
}

fn growth_cell(vals: &[(String, f64)], year: &str) -> Cell {
    let maybe = vals.iter().find(|(yr, _)| yr == year);
    let text = maybe
        .map(|(_, v)| format!("{:+.2}%", v))
        .unwrap_or_else(|| "N/A".to_string());
    let cell = Cell::new(text).set_alignment(CellAlignment::Right);
    match maybe {
        Some((_, v)) if *v < 0.0 => cell.fg(Color::Red),
        Some((_, v)) if *v > 0.0 => cell.fg(Color::Green),
        _ => cell,
    }
}

fn x_cell(vals: &[(String, f64)], year: &str) -> Cell {
    let text = vals
        .iter()
        .find(|(yr, _)| yr == year)
        .map(|(_, v)| format!("{:.2}x", v))
        .unwrap_or_else(|| "N/A".to_string());
    Cell::new(text).set_alignment(CellAlignment::Right)
}

fn label_cell(text: &str) -> Cell {
    Cell::new(text).add_attribute(Attribute::Bold)
}

fn section_header_row(years: &[String], title: &str) -> Vec<Cell> {
    let mut row = vec![Cell::new(title).add_attribute(Attribute::Bold)];
    for _ in years { row.push(Cell::new("")); }
    row
}

pub fn print(ticker: &str, data: parse::Data) {
    let roe_val = roe(&data.netincomeloss, &data.stockholdersequity);
    let roa_val = roa(&data.netincomeloss, &data.assets);
    let opm_val = opm(&data.operatingincomeloss, &data.revenues);
    let npm_val = npm(&data.netincomeloss, &data.revenues);
    let der_val = der(&data.liabilities, &data.stockholdersequity);
    let er_val  = er(&data.stockholdersequity, &data.assets);
    let at_val  = at(&data.revenues, &data.assets);

    let dr_val  = debt_ratio(&data.liabilities, &data.assets);
    let em_val  = equity_multiplier(&data.assets, &data.stockholdersequity);

    let rev_growth = yoy_growth(&data.revenues);
    let ni_growth  = yoy_growth(&data.netincomeloss);

    let dupont_val = dupont(&npm_val, &at_val, &em_val);

    let mut years: Vec<String> = Vec::new();
    for ds in [
        data.revenues.as_slice(), data.netincomeloss.as_slice(),
        data.operatingincomeloss.as_slice(), data.assets.as_slice(),
        data.liabilities.as_slice(), data.stockholdersequity.as_slice(),
    ] {
        for (date, _) in ds {
            if date.len() >= 4 { years.push(date[..4].to_string()); }
        }
    }
    for ds in [
        roe_val.as_slice(), roa_val.as_slice(), opm_val.as_slice(), npm_val.as_slice(),
        der_val.as_slice(), er_val.as_slice(),  at_val.as_slice(),
        dr_val.as_slice(),  em_val.as_slice(),
        rev_growth.as_slice(), ni_growth.as_slice(), dupont_val.as_slice(),
    ] {
        for (yr, _) in ds { years.push(yr.clone()); }
    }
    years.sort();
    years.dedup();

    if years.is_empty() {
        println!("=== {} ===", ticker);
        println!("  표시할 재무 데이터가 없습니다.");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    {
        let mut header = vec![Cell::new(format!("  {}  재무제표", ticker))
            .add_attribute(Attribute::Bold)
            .fg(Color::Green)];
        for y in &years {
            header.push(Cell::new(y).add_attribute(Attribute::Bold).set_alignment(CellAlignment::Center));
        }
        table.set_header(header);
    }

    table.add_row(section_header_row(&years, "▸ 손익"));
    for (label, vals) in [
        ("매출",    data.revenues.as_slice()),
        ("영업이익", data.operatingincomeloss.as_slice()),
        ("순이익",  data.netincomeloss.as_slice()),
    ] {
        let mut row = vec![label_cell(label)];
        for y in &years { row.push(raw_cell(vals, y)); }
        table.add_row(row);
    }

    table.add_row(section_header_row(&years, "▸ 성장률"));
    {
        let mut row = vec![label_cell("매출 성장률 (YoY)")];
        for y in &years { row.push(growth_cell(&rev_growth, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("순이익 성장률 (YoY)")];
        for y in &years { row.push(growth_cell(&ni_growth, y)); }
        table.add_row(row);
    }

    table.add_row(section_header_row(&years, "▸ 재무상태"));
    for (label, vals) in [
        ("총자산",   data.assets.as_slice()),
        ("부채",     data.liabilities.as_slice()),
        ("자기자본", data.stockholdersequity.as_slice()),
    ] {
        let mut row = vec![label_cell(label)];
        for y in &years { row.push(raw_cell(vals, y)); }
        table.add_row(row);
    }

    table.add_row(section_header_row(&years, "▸ 수익성 지표"));
    for (label, vals) in [
        ("ROE  자기자본이익률", roe_val.as_slice()),
        ("ROA  총자산이익률",   roa_val.as_slice()),
        ("OPM  영업이익률",     opm_val.as_slice()),
        ("NPM  순이익률",       npm_val.as_slice()),
    ] {
        let mut row = vec![label_cell(label)];
        for y in &years { row.push(pct_cell(vals, y)); }
        table.add_row(row);
    }

    table.add_row(section_header_row(&years, "▸ 안정성 지표"));
    {
        let mut row = vec![label_cell("DER  부채자본비율")];
        for y in &years { row.push(pct_cell(&der_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("DR   총부채비율")];
        for y in &years { row.push(pct_cell(&dr_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("ER   자기자본비율")];
        for y in &years { row.push(pct_cell(&er_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("AT   자산회전율")];
        for y in &years { row.push(x_cell(&at_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("EM   재무레버리지")];
        for y in &years { row.push(x_cell(&em_val, y)); }
        table.add_row(row);
    }

    table.add_row(section_header_row(&years, "▸ 듀폰 분석  (NPM × AT × EM = ROE)"));
    {
        let mut row = vec![label_cell("NPM  순이익률")];
        for y in &years { row.push(pct_cell(&npm_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("AT   자산회전율")];
        for y in &years { row.push(x_cell(&at_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("EM   재무레버리지")];
        for y in &years { row.push(x_cell(&em_val, y)); }
        table.add_row(row);
    }
    {
        let mut row = vec![label_cell("ROE  검증값")];
        for y in &years { row.push(pct_cell(&dupont_val, y)); }
        table.add_row(row);
    }

    println!("\n{table}");
}
