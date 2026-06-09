mod calculate;
mod extract;
mod extract_edinet;
mod parse;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("How to use: {} <ticker>", args[0]);
        eprintln!("  U.S. Stocks: {} AAPL", args[0]);
        eprintln!("  J.P. Stocks: {} JP:7203  (4-digit stock code)", args[0]);
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
            eprintln!("The ticker cannot be found: {}", input);
            std::process::exit(1);
        }
        extract::get_company_facts(&input, &cik);
        let data = parse::Data::new(&input);
        calculate::print(&input, data);
    }
}
