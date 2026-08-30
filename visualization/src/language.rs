use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    ZhCn,
    EnUs,
}

impl Language {
    pub fn code(&self) -> &str {
        match self {
            Language::ZhCn => "zh-CN",
            Language::EnUs => "en-US",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageResource {
    pub title: &'static str,
    pub legend_open: &'static str,
    pub legend_high: &'static str,
    pub legend_low: &'static str,
    pub legend_close: &'static str,
    pub legend_volume: &'static str,
    pub tooltip_date: &'static str,
    pub tooltip_open: &'static str,
    pub tooltip_high: &'static str,
    pub tooltip_low: &'static str,
    pub tooltip_close: &'static str,
    pub tooltip_volume: &'static str,
    pub no_data: &'static str,
    pub zoom_in: &'static str,
    pub zoom_out: &'static str,
    pub reset: &'static str,
    pub indicator_ma: &'static str,
    pub indicator_ema: &'static str,
    pub indicator_boll: &'static str,
    pub indicator_macd: &'static str,
    pub indicator_rsi: &'static str,
    pub indicator_kdj: &'static str,
    pub indicator_sar: &'static str,
    pub indicator_cci: &'static str,
    pub indicator_obv: &'static str,
    pub axis_price: &'static str,
    pub axis_volume: &'static str,
    pub axis_date: &'static str,
    pub axis_percentage: &'static str,
    pub scroll_to_zoom: &'static str,
    pub drag_to_pan: &'static str,
    pub hover_for_details: &'static str,
}

pub const ZH_CN_RESOURCE: LanguageResource = LanguageResource {
    title: "K线图",
    legend_open: "开盘",
    legend_high: "最高",
    legend_low: "最低",
    legend_close: "收盘",
    legend_volume: "成交量",
    tooltip_date: "日期",
    tooltip_open: "开盘",
    tooltip_high: "最高",
    tooltip_low: "最低",
    tooltip_close: "收盘",
    tooltip_volume: "成交量",
    no_data: "暂无数据",
    zoom_in: "放大",
    zoom_out: "缩小",
    reset: "重置",
    indicator_ma: "移动平均线",
    indicator_ema: "指数移动平均线",
    indicator_boll: "布林带",
    indicator_macd: "指数平滑异同移动平均线",
    indicator_rsi: "相对强弱指标",
    indicator_kdj: "随机指标",
    indicator_sar: "抛物线指标",
    indicator_cci: "商品通道指标",
    indicator_obv: "能量潮指标",
    axis_price: "价格",
    axis_volume: "成交量",
    axis_date: "日期",
    axis_percentage: "百分比",
    scroll_to_zoom: "滚动缩放",
    drag_to_pan: "拖拽平移",
    hover_for_details: "悬停查看详情",
};

pub const EN_US_RESOURCE: LanguageResource = LanguageResource {
    title: "K-Line Chart",
    legend_open: "Open",
    legend_high: "High",
    legend_low: "Low",
    legend_close: "Close",
    legend_volume: "Volume",
    tooltip_date: "Date",
    tooltip_open: "Open",
    tooltip_high: "High",
    tooltip_low: "Low",
    tooltip_close: "Close",
    tooltip_volume: "Volume",
    no_data: "No Data",
    zoom_in: "Zoom In",
    zoom_out: "Zoom Out",
    reset: "Reset",
    indicator_ma: "Moving Average",
    indicator_ema: "Exponential Moving Average",
    indicator_boll: "Bollinger Bands",
    indicator_macd: "MACD",
    indicator_rsi: "Relative Strength Index",
    indicator_kdj: "Stochastic Oscillator",
    indicator_sar: "Parabolic SAR",
    indicator_cci: "Commodity Channel Index",
    indicator_obv: "On-Balance Volume",
    axis_price: "Price",
    axis_volume: "Volume",
    axis_date: "Date",
    axis_percentage: "Percentage",
    scroll_to_zoom: "Scroll to Zoom",
    drag_to_pan: "Drag to Pan",
    hover_for_details: "Hover for Details",
};

impl LanguageResource {
    pub fn from_language(lang: &Language) -> &'static LanguageResource {
        match lang {
            Language::ZhCn => &ZH_CN_RESOURCE,
            Language::EnUs => &EN_US_RESOURCE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_code() {
        assert_eq!(Language::ZhCn.code(), "zh-CN");
        assert_eq!(Language::EnUs.code(), "en-US");
    }

    #[test]
    fn test_language_resource_zh_cn() {
        let resource = LanguageResource::from_language(&Language::ZhCn);
        assert_eq!(resource.title, "K线图");
        assert_eq!(resource.legend_volume, "成交量");
        assert_eq!(resource.indicator_ma, "移动平均线");
        assert_eq!(resource.indicator_macd, "指数平滑异同移动平均线");
        assert_eq!(resource.axis_price, "价格");
        assert_eq!(resource.scroll_to_zoom, "滚动缩放");
    }

    #[test]
    fn test_language_resource_en_us() {
        let resource = LanguageResource::from_language(&Language::EnUs);
        assert_eq!(resource.title, "K-Line Chart");
        assert_eq!(resource.legend_volume, "Volume");
        assert_eq!(resource.indicator_ma, "Moving Average");
        assert_eq!(resource.axis_price, "Price");
        assert_eq!(resource.hover_for_details, "Hover for Details");
    }

    #[test]
    fn test_language_default() {
        assert!(matches!(Language::default(), Language::ZhCn));
    }
}
