# company-analyzer

SEC EDGAR API에서 기업 재무 데이터를 가져와 미국 상장 기업의 5개년 재무 분석을 출력하는 Rust 코드입니다.

## 동작 방식

1. SEC EDGAR에서 `company_tickers.json`을 받아 티커 심볼을 CIK(Central Index Key)로 변환
2. `data.sec.gov`에서 해당 기업의 XBRL facts JSON 전체를 가져옴
3. 연간 10-K 공시를 파싱하여 항목별로 가장 적합한 GAAP 태그를 선택하고 회계연도별로 중복 제거
4. 재무 지표를 계산하고 `comfy-table`로 포맷된 테이블 출력

JSON 파일은 `~/ca_json/`에 캐시되며 24시간이 지난 경우에만 재요청합니다.

## 지표

### 원시 재무 데이터 (정확한 수치, 콤마 구분)

| 항목 | GAAP 태그 |
|---|---|
| 매출 | `Revenues`, `RevenueFromContractWithCustomerExcludingAssessedTax`, `SalesRevenueNet` |
| 순이익 | `NetIncomeLoss`, `NetIncomeLossAvailableToCommonStockholdersBasic` |
| 영업이익 | `OperatingIncomeLoss`, `OperatingLoss` |
| 총자산 | `Assets` |
| 부채 | `Liabilities`, `LiabilitiesCurrent` |
| 자기자본 | `StockholdersEquity` (없으면 총자산 − 부채로 대체) |

### 성장률

| 지표 | 계산식 | 의미 |
|---|---|---|
| 매출 성장률 (YoY) | (올해 매출 − 작년 매출) / 작년 매출 × 100 | 전년 대비 매출 증감 |
| 순이익 성장률 (YoY) | (올해 순이익 − 작년 순이익) / 작년 순이익 × 100 | 전년 대비 이익 증감 |

### 수익성 지표

| 지표 | 계산식 | 의미 |
|---|---|---|
| ROE | 순이익 / 자기자본 × 100 | 자기자본 대비 수익 창출 능력 |
| ROA | 순이익 / 총자산 × 100 | 자산 대비 수익 창출 효율 |
| OPM | 영업이익 / 매출 × 100 | 핵심 영업 마진 |
| NPM | 순이익 / 매출 × 100 | 모든 비용 차감 후 최종 마진 |

### 안정성 지표

| 지표 | 계산식 | 의미 |
|---|---|---|
| DER | 부채 / 자기자본 × 100 | 자기자본 1원당 부채 비율 |
| DR | 부채 / 총자산 × 100 | 총자산 중 부채 비중 (100% 초과 불가) |
| ER | 자기자본 / 총자산 × 100 | 자본 구조 안정성 |
| AT | 매출 / 총자산 | 자산 활용 효율 (자산회전율) |
| EM | 총자산 / 자기자본 | 재무레버리지 — 클수록 부채 의존도 높음 |

### 듀폰 분석 (DuPont Analysis)

ROE를 세 가지 동인으로 분해합니다: `ROE = NPM × AT × EM`

| 구성요소 | 의미 |
|---|---|
| NPM (순이익률) | 수익성이 ROE를 견인하는가? |
| AT (자산회전율) | 자산 효율이 ROE를 견인하는가? |
| EM (재무레버리지) | 단순히 부채가 ROE를 부풀리는가? |

ROE 검증값은 세 구성요소의 곱으로 재계산되어 원래 ROE와 대조할 수 있습니다.

## 설치

Rust와 Cargo가 필요합니다 ([rustup.rs](https://rustup.rs)).

```bash
git clone https://github.com/your-username/company-analyzer
cd company-analyzer
cargo build --release
```

빌드된 바이너리는 `target/release/company-analyzer`에 생성됩니다.

## 사용법

```bash
./company-analyzer AAPL
./company-analyzer MSFT
./company-analyzer NVDA
./company-analyzer HPE
```

티커는 SEC EDGAR에 등록된 심볼과 동일해야 합니다 (보통 NYSE/NASDAQ 티커와 같습니다).

## 프로젝트 구조

```
src/
├── main.rs        # 진입점 및 인수 처리
├── extract.rs     # SEC EDGAR HTTP 클라이언트 및 로컬 JSON 캐시
├── parse.rs       # XBRL JSON 파싱 및 5개년 데이터 추출
└── calculate.rs   # 지표 계산 및 테이블 출력
```

## Dependency

```toml
[dependencies]
reqwest = { version = "0.11", features = ["blocking"] }
serde_json = "1.0"
comfy-table = "7"
```

## 한계

- **미국 기업 한정.** SEC EDGAR는 미국 SEC에 공시하는 기업만 커버합니다. 해외 상장 기업은 조회되지 않습니다.
- **GAAP 태그 한정.** `us-gaap` XBRL 태그만 파싱합니다. IFRS 기준 또는 비표준 태그를 사용하는 기업은 일부 지표가 누락될 수 있습니다.
- **단일 태그 선택.** 동일 개념에 여러 GAAP 태그가 존재할 경우 (예: ASC 606 기준 변경으로 2018년 매출 태그 변경), 가장 최근 데이터를 가진 태그를 선택합니다. 이전 태그를 사용한 과거 공시는 제외될 수 있습니다.
- **5개년 윈도우.** 항목별 최근 10-K 5개 연도만 표시됩니다.
- **회계연도 시차.** 12월 결산 기업은 익년 2~3월, 애플처럼 9월 결산 기업은 익년 11월경에야 최신 연도 데이터가 SEC에 등록됩니다.

## 라이선스

MIT
