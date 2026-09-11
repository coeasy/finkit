use finkit::returns::ReturnKind;
use finkit_factor_analysis::analysis::{factor_weights, WeightConfig};
use finkit_factor_analysis::data::{AssetId, GroupId, PanelIndex, ResearchFrame};
use finkit_factor_analysis::prepare::{
    compute_forward_returns, quantize_factor, ForwardReturnConfig, QuantizeConfig,
};

fn make_frame(last_price_asset_1: f64, last_group_asset_1: u32) -> ResearchFrame {
    let index = PanelIndex::new(
        vec![1, 1, 2, 2, 3, 3, 4, 4],
        vec![
            AssetId(1),
            AssetId(2),
            AssetId(1),
            AssetId(2),
            AssetId(1),
            AssetId(2),
            AssetId(1),
            AssetId(2),
        ],
    )
    .unwrap();
    let mut frame = ResearchFrame::new(index);
    frame
        .add_numeric(
            "price",
            "market",
            vec![10.0, 20.0, 11.0, 21.0, 12.0, 22.0, last_price_asset_1, 23.0],
        )
        .unwrap();
    frame
        .add_numeric(
            "factor",
            "factor",
            vec![1.0, -1.0, 1.5, -1.5, 2.0, -2.0, 3.0, -3.0],
        )
        .unwrap();
    frame
        .add_group(
            "group",
            vec![
                GroupId(1),
                GroupId(2),
                GroupId(1),
                GroupId(2),
                GroupId(1),
                GroupId(2),
                GroupId(last_group_asset_1),
                GroupId(2),
            ],
        )
        .unwrap();
    frame
}

#[test]
fn future_price_mutation_only_changes_labels_within_the_requested_horizon() {
    let original = make_frame(13.0, 1);
    let mutated = make_frame(1300.0, 1);
    let config = ForwardReturnConfig::new(vec![1], ReturnKind::Arithmetic).unwrap();
    let a = compute_forward_returns(&original, "price", &config).unwrap();
    let b = compute_forward_returns(&mutated, "price", &config).unwrap();

    // Rows at dates 1 and 2 cannot observe the price mutation at date 4.
    assert_eq!(a[&1][0], b[&1][0]);
    assert_eq!(a[&1][2], b[&1][2]);
    // Date 3 -> date 4 is the only one-day label for asset 1 that may change.
    assert_ne!(a[&1][4], b[&1][4]);
}

#[test]
fn future_group_mutation_does_not_change_historical_quantiles_or_weights() {
    let original = make_frame(13.0, 1);
    let mutated = make_frame(13.0, 99);
    let quantize = QuantizeConfig {
        quantiles: 2,
        by_group: Some("group".to_string()),
        zero_aware: false,
    };
    let a = quantize_factor(&original, "factor", &quantize).unwrap();
    let b = quantize_factor(&mutated, "factor", &quantize).unwrap();
    assert_eq!(&a[..6], &b[..6]);

    let weights = WeightConfig {
        demeaned: true,
        group_adjust: None,
        equal_weight: false,
    };
    let wa = factor_weights(&original, "factor", &weights).unwrap();
    let wb = factor_weights(&mutated, "factor", &weights).unwrap();
    assert_eq!(&wa[..6], &wb[..6]);
}

#[test]
fn changing_future_factor_values_does_not_rewrite_past_cross_sections() {
    let a = make_frame(13.0, 1);
    let mut b = make_frame(13.0, 1);
    let mut changed = b.column("factor").unwrap().to_vec();
    changed[6] = -1000.0;
    changed[7] = 1000.0;
    b.add_numeric("future_factor", "factor", changed).unwrap();

    let config = QuantizeConfig {
        quantiles: 2,
        by_group: None,
        zero_aware: false,
    };
    let qa = quantize_factor(&a, "factor", &config).unwrap();
    let qb = quantize_factor(&b, "future_factor", &config).unwrap();
    assert_eq!(&qa[..6], &qb[..6]);
}