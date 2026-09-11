use clap::Parser;
use finkit_factor_analysis::{AssetId, FactorStudy, PanelIndex, QuantizeConfig, ResearchFrame};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "finkit-factor-study")]
#[command(about = "Run panel-aware factor research and emit a JSON report")]
struct Args {
    /// CSV with columns: timestamp,asset,factor,price[,group]
    #[arg(short, long)]
    input: PathBuf,
    /// Comma-separated forward-return horizons in bars.
    #[arg(long, default_value = "1,5,10")]
    periods: String,
    #[arg(long, default_value_t = 5)]
    quantiles: u16,
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[derive(Debug)]
struct Row {
    timestamp: i64,
    asset: String,
    factor: f64,
    price: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let periods: Vec<usize> = args
        .periods
        .split(',')
        .map(str::trim)
        .map(str::parse)
        .collect::<Result<_, _>>()?;

    let mut reader = csv::Reader::from_path(&args.input)?;
    let headers = reader.headers()?.clone();
    let timestamp_idx = headers
        .iter()
        .position(|h| h == "timestamp")
        .ok_or("missing timestamp column")?;
    let asset_idx = headers
        .iter()
        .position(|h| h == "asset")
        .ok_or("missing asset column")?;
    let factor_idx = headers
        .iter()
        .position(|h| h == "factor")
        .ok_or("missing factor column")?;
    let price_idx = headers
        .iter()
        .position(|h| h == "price")
        .ok_or("missing price column")?;

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        rows.push(Row {
            timestamp: record
                .get(timestamp_idx)
                .ok_or("missing timestamp value")?
                .parse()?,
            asset: record
                .get(asset_idx)
                .ok_or("missing asset value")?
                .to_string(),
            factor: record
                .get(factor_idx)
                .ok_or("missing factor value")?
                .parse()?,
            price: record
                .get(price_idx)
                .ok_or("missing price value")?
                .parse()?,
        });
    }
    rows.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.asset.cmp(&b.asset)));

    let mut asset_ids = BTreeMap::<String, AssetId>::new();
    let mut next_id = 0u32;
    for row in &rows {
        asset_ids.entry(row.asset.clone()).or_insert_with(|| {
            let id = AssetId(next_id);
            next_id += 1;
            id
        });
    }

    let index = PanelIndex::new(
        rows.iter().map(|row| row.timestamp).collect(),
        rows.iter().map(|row| asset_ids[&row.asset]).collect(),
    )?;
    let mut frame = ResearchFrame::new(index);
    frame.add_numeric(
        "factor",
        "factor",
        rows.iter().map(|row| row.factor).collect(),
    )?;
    frame.add_numeric(
        "price",
        "market",
        rows.iter().map(|row| row.price).collect(),
    )?;

    let report = FactorStudy::new(&frame, "factor", "price", periods)
        .quantize_config(QuantizeConfig {
            quantiles: args.quantiles,
            by_group: None,
            zero_aware: false,
        })
        .full_report()?;
    let json = report.to_json_pretty()?;
    if let Some(output) = args.output {
        fs::write(output, json)?;
    } else {
        println!("{json}");
    }
    Ok(())
}
