mod calculate;
mod extract;
mod extract_edinet;
mod parse;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("사용법: {} <ticker>", args[0]);
        eprintln!("  미국 주식: {} AAPL", args[0]);
        eprintln!("  일본 주식: {} JP:7203  (증권코드 4자리)", args[0]);
        std::process::exit(1);
    }

    let input = args[1].to_uppercase();

    if let Some(sec_code) = input.strip_prefix("JP:") {
        let data = extract_edinet::get_data(sec_code);
        calculate::print(&input, data);
    } else {
        extract::get_company_tickers();
        let cik = extract::return_ticker(&input);
        if cik.is_empty() {
            eprintln!("티커를 찾을 수 없습니다: {}", input);
            std::process::exit(1);
        }
        extract::get_company_facts(&input, &cik);
        let data = parse::Data::new(&input);
        calculate::print(&input, data);
    }
}
