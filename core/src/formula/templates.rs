use std::collections::HashMap;

/// 公式模板分类
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TemplateCategory {
    MovingAverage,
    Trend,
    Oscillator,
    Volume,
    Pattern,
    Strategy,
    Classic,
    TDXClassic,
    THSSmartSelect,
    DZHMoneyFlow,
    FoxTrader,
    EMClassic,
}

/// 公式模板
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaTemplate {
    pub name: String,
    pub description: String,
    pub category: TemplateCategory,
    pub source: String,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub parameters: Vec<(String, f64, f64, f64)>,
}

/// 公式模板库
pub struct FormulaTemplates {
    templates: HashMap<String, FormulaTemplate>,
    categories: HashMap<TemplateCategory, Vec<String>>,
}

impl Default for FormulaTemplates {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaTemplates {
    pub fn new() -> Self {
        let builtin = init_builtin_templates();
        let mut categories: HashMap<TemplateCategory, Vec<String>> = HashMap::new();
        for (key, tmpl) in &builtin {
            categories
                .entry(tmpl.category.clone())
                .or_default()
                .push(key.clone());
        }
        Self {
            templates: builtin,
            categories,
        }
    }

    pub fn get(&self, name: &str) -> Option<&FormulaTemplate> {
        self.templates.get(name)
    }

    pub fn get_by_category(&self, category: &TemplateCategory) -> Vec<&FormulaTemplate> {
        self.categories
            .get(category)
            .map(|names| names.iter().filter_map(|n| self.templates.get(n)).collect())
            .unwrap_or_default()
    }

    pub fn search(&self, keyword: &str) -> Vec<&FormulaTemplate> {
        let kw = keyword.to_lowercase();
        self.templates
            .values()
            .filter(|t| {
                t.name.to_lowercase().contains(&kw)
                    || t.description.to_lowercase().contains(&kw)
                    || t.source.to_lowercase().contains(&kw)
            })
            .collect()
    }

    pub fn list_all(&self) -> Vec<&FormulaTemplate> {
        self.templates.values().collect()
    }

    pub fn categories() -> Vec<TemplateCategory> {
        vec![
            TemplateCategory::MovingAverage,
            TemplateCategory::Trend,
            TemplateCategory::Oscillator,
            TemplateCategory::Volume,
            TemplateCategory::Pattern,
            TemplateCategory::Strategy,
            TemplateCategory::Classic,
            TemplateCategory::TDXClassic,
            TemplateCategory::THSSmartSelect,
            TemplateCategory::DZHMoneyFlow,
            TemplateCategory::FoxTrader,
            TemplateCategory::EMClassic,
        ]
    }

    /// Validate that all function references in a template are registered.
    /// Returns a list of unresolved function names.
    pub fn validate_template(source: &str, registered_functions: &std::collections::HashSet<String>) -> Vec<String> {
        let mut unresolved = Vec::new();
        let known_data = ["OPEN", "HIGH", "LOW", "CLOSE", "VOLUME", "AMOUNT", "O", "H", "L", "C", "V", "A", "VOL"];
        let keywords = ["IF", "THEN", "ELSE", "AND", "OR", "XOR", "NOT", "FOR", "WHILE", "DO", "END", "TO", "TRUE", "FALSE", "PARAMS"];

        for token in source.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if token.is_empty() || token.chars().next().is_none_or(|c| c.is_ascii_digit()) {
                continue;
            }
            let upper = token.to_uppercase();
            if known_data.contains(&upper.as_str()) || keywords.contains(&upper.as_str()) {
                continue;
            }
            if source.contains(&format!("{}(", token))
                && !registered_functions.contains(&upper)
                && !unresolved.contains(&upper)
            {
                unresolved.push(upper);
            }
        }
        unresolved
    }
}

fn init_builtin_templates() -> HashMap<String, FormulaTemplate> {
    let mut map = HashMap::new();

    // ========== 均线系统 ==========
    map.insert(
        "ma_cross".to_string(),
        FormulaTemplate {
            name: "均线金叉死叉".to_string(),
            description: "短周期均线上穿长周期均值为买入信号，下穿为卖出信号".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "MA5:=MA(CLOSE,SHORT); MA10:=MA(CLOSE,LONG); CROSS(MA5,MA10)".to_string(),
            parameters: vec![
                ("SHORT".to_string(), 2.0, 60.0, 5.0),
                ("LONG".to_string(), 5.0, 120.0, 10.0),
            ],
        },
    );

    map.insert(
        "ema_cross".to_string(),
        FormulaTemplate {
            name: "EMA金叉死叉".to_string(),
            description: "指数移动平均线交叉信号".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "E1:=EMA(CLOSE,SHORT); E2:=EMA(CLOSE,LONG); CROSS(E1,E2)".to_string(),
            parameters: vec![
                ("SHORT".to_string(), 5.0, 60.0, 12.0),
                ("LONG".to_string(), 10.0, 200.0, 26.0),
            ],
        },
    );

    map.insert(
        "ma_multi".to_string(),
        FormulaTemplate {
            name: "多均线多头排列".to_string(),
            description: "短期均线在长期均线之上，表示多头排列".to_string(),
            category: TemplateCategory::MovingAverage,
            source:
                "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20"
                    .to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "sma_ribbon".to_string(),
        FormulaTemplate {
            name: "均线带".to_string(),
            description: "多条均线形成的带状区域，判断趋势方向".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "M1:=MA(CLOSE,5); M2:=MA(CLOSE,10); M3:=MA(CLOSE,20); M4:=MA(CLOSE,60); M1-M4"
                .to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "bull_bear_line".to_string(),
        FormulaTemplate {
            name: "多空线".to_string(),
            description: "基于均线的多空分界线".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "DKX:=(3*MA(CLOSE,9)+MA(CLOSE,18)+MA(CLOSE,36))/5; CROSS(CLOSE,DKX)"
                .to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "ma_deviation".to_string(),
        FormulaTemplate {
            name: "均线偏离度".to_string(),
            description: "收盘价与均线的偏离百分比".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "MA20:=MA(CLOSE,20); (CLOSE-MA20)/MA20*100".to_string(),
            parameters: vec![("N".to_string(), 5.0, 120.0, 20.0)],
        },
    );

    map.insert(
        "wma_cross".to_string(),
        FormulaTemplate {
            name: "加权均线交叉".to_string(),
            description: "加权移动平均线交叉信号".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "W1:=WMA(CLOSE,SHORT); W2:=WMA(CLOSE,LONG); CROSS(W1,W2)".to_string(),
            parameters: vec![
                ("SHORT".to_string(), 5.0, 30.0, 10.0),
                ("LONG".to_string(), 20.0, 120.0, 30.0),
            ],
        },
    );

    map.insert(
        "hma_trend".to_string(),
        FormulaTemplate {
            name: "赫尔均线趋势".to_string(),
            description: "赫尔移动平均线趋势判断".to_string(),
            category: TemplateCategory::MovingAverage,
            source: "HMA:=2*EMA(CLOSE,SHORT/2)-EMA(CLOSE,SHORT); CLOSE>HMA".to_string(),
            parameters: vec![("N".to_string(), 5.0, 60.0, 21.0)],
        },
    );

    // ========== MACD系列 ==========
    map.insert("macd_golden_cross".to_string(), FormulaTemplate {
        name: "MACD金叉".to_string(),
        description: "DIF上穿DEA形成MACD金叉买入信号".to_string(),
        category: TemplateCategory::Trend,
        source: "DIF:=EMA(CLOSE,SHORT)-EMA(CLOSE,LONG); DEA:=EMA(DIF,M); MACD:=(DIF-DEA)*2; CROSS(DIF,DEA)".to_string(),
        parameters: vec![("SHORT".to_string(), 6.0, 24.0, 12.0), ("LONG".to_string(), 12.0, 60.0, 26.0), ("M".to_string(), 4.0, 20.0, 9.0)],
    });

    map.insert("macd_death_cross".to_string(), FormulaTemplate {
        name: "MACD死叉".to_string(),
        description: "DIF下穿DEA形成MACD死叉卖出信号".to_string(),
        category: TemplateCategory::Trend,
        source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; CROSS(DEA,DIF)".to_string(),
        parameters: vec![],
    });

    map.insert("macd_divergence_bottom".to_string(), FormulaTemplate {
        name: "MACD底背离".to_string(),
        description: "股价创新低但MACD没有创新低，看涨背离".to_string(),
        category: TemplateCategory::Trend,
        source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; REF(MACD,1)<MACD AND MACD<0".to_string(),
        parameters: vec![],
    });

    map.insert(
        "macd_zero_cross".to_string(),
        FormulaTemplate {
            name: "MACD零轴穿越".to_string(),
            description: "DIF穿越零轴的趋势确认信号".to_string(),
            category: TemplateCategory::Trend,
            source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); CROSS(DIF,0)".to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "macd_red_green".to_string(),
        FormulaTemplate {
            name: "MACD红绿柱".to_string(),
            description: "MACD柱状图红绿柱变化".to_string(),
            category: TemplateCategory::Trend,
            source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); (DIF-DEA)*2".to_string(),
            parameters: vec![],
        },
    );

    // ========== KDJ ==========
    map.insert("kdj_golden_cross".to_string(), FormulaTemplate {
        name: "KDJ金叉".to_string(),
        description: "K线上穿D线形成金叉买入信号".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSV:=(CLOSE-LLV(LOW,N))/(HHV(HIGH,N)-LLV(LOW,N))*100; K:=SMA(RSV,M1,1); D:=SMA(K,M2,1); J:=3*K-2*D; CROSS(K,D)".to_string(),
        parameters: vec![("N".to_string(), 3.0, 30.0, 9.0), ("M1".to_string(), 2.0, 10.0, 3.0), ("M2".to_string(), 2.0, 10.0, 3.0)],
    });

    map.insert("kdj_death_cross".to_string(), FormulaTemplate {
        name: "KDJ死叉".to_string(),
        description: "K线下穿D线形成死叉卖出信号".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); CROSS(D,K)".to_string(),
        parameters: vec![],
    });

    map.insert("kdj_overbought".to_string(), FormulaTemplate {
        name: "KDJ超买".to_string(),
        description: "J值超过80进入超买区域".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J>80".to_string(),
        parameters: vec![],
    });

    map.insert("kdj_oversold".to_string(), FormulaTemplate {
        name: "KDJ超卖".to_string(),
        description: "J值低于20进入超卖区域".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J<20".to_string(),
        parameters: vec![],
    });

    // ========== RSI ==========
    map.insert("rsi_golden_cross".to_string(), FormulaTemplate {
        name: "RSI金叉".to_string(),
        description: "短期RSI上穿长期RSI".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSI1:=SMA(MAX(CLOSE-REF(CLOSE,1),0),SHORT,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),SHORT,1)*100; RSI2:=SMA(MAX(CLOSE-REF(CLOSE,1),0),LONG,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),LONG,1)*100; CROSS(RSI1,RSI2)".to_string(),
        parameters: vec![("SHORT".to_string(), 3.0, 14.0, 6.0), ("LONG".to_string(), 12.0, 30.0, 12.0)],
    });

    map.insert("rsi_overbought".to_string(), FormulaTemplate {
        name: "RSI超买".to_string(),
        description: "RSI超过70进入超买区域".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),N,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),N,1)*100; RSI>70".to_string(),
        parameters: vec![("N".to_string(), 6.0, 30.0, 14.0)],
    });

    map.insert("rsi_oversold".to_string(), FormulaTemplate {
        name: "RSI超卖".to_string(),
        description: "RSI低于30进入超卖区域".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),N,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),N,1)*100; RSI<30".to_string(),
        parameters: vec![("N".to_string(), 6.0, 30.0, 14.0)],
    });

    map.insert("rsi_divergence".to_string(), FormulaTemplate {
        name: "RSI背离".to_string(),
        description: "RSI与价格出现背离信号".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; REF(RSI,1)<RSI AND CLOSE<REF(CLOSE,1)".to_string(),
        parameters: vec![],
    });

    // ========== 布林带 ==========
    map.insert("boll_break_up".to_string(), FormulaTemplate {
        name: "布林带突破上轨".to_string(),
        description: "收盘价突破布林带上轨".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MID:=MA(CLOSE,N); UPPER:=MID+STD(CLOSE,N)*2; LOWER:=MID-STD(CLOSE,N)*2; CROSS(CLOSE,UPPER)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("boll_break_down".to_string(), FormulaTemplate {
        name: "布林带跌破下轨".to_string(),
        description: "收盘价跌破布林带下轨".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; CROSS(LOWER,CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("boll_squeeze".to_string(), FormulaTemplate {
        name: "布林带缩口".to_string(),
        description: "布林带上下轨收窄，预示即将突破".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; (UPPER-LOWER)/MID*100<10".to_string(),
        parameters: vec![],
    });

    map.insert(
        "boll_mid_support".to_string(),
        FormulaTemplate {
            name: "布林带中轨支撑".to_string(),
            description: "回调至布林带中轨获得支撑".to_string(),
            category: TemplateCategory::Oscillator,
            source: "MID:=MA(CLOSE,20); CLOSE>MID AND REF(CLOSE,1)<MID".to_string(),
            parameters: vec![],
        },
    );

    // ========== 成交量指标 ==========
    map.insert(
        "volume_price_rise".to_string(),
        FormulaTemplate {
            name: "量价齐升".to_string(),
            description: "成交量和价格同时上涨".to_string(),
            category: TemplateCategory::Volume,
            source: "CLOSE>REF(CLOSE,1) AND VOLUME>REF(VOLUME,1)".to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "volume_shrink_back".to_string(),
        FormulaTemplate {
            name: "缩量回调".to_string(),
            description: "价格回调但成交量萎缩，支撑有效".to_string(),
            category: TemplateCategory::Volume,
            source: "CLOSE<REF(CLOSE,1) AND VOLUME<REF(VOLUME,1)".to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "volume_breakout".to_string(),
        FormulaTemplate {
            name: "放量突破".to_string(),
            description: "成交量显著放大伴随价格突破".to_string(),
            category: TemplateCategory::Volume,
            source: "MAVOL:=MA(VOLUME,N); VOLUME>MAVOL*2 AND CLOSE>REF(HHV(HIGH,N),1)".to_string(),
            parameters: vec![("N".to_string(), 5.0, 60.0, 20.0)],
        },
    );

    map.insert(
        "volume_ratio".to_string(),
        FormulaTemplate {
            name: "量比指标".to_string(),
            description: "当前成交量与平均成交量的比值".to_string(),
            category: TemplateCategory::Volume,
            source: "MAVOL:=MA(VOLUME,N); VOLUME/MAVOL".to_string(),
            parameters: vec![("N".to_string(), 3.0, 30.0, 5.0)],
        },
    );

    map.insert("obv_trend".to_string(), FormulaTemplate {
        name: "OBV能量潮".to_string(),
        description: "On Balance Volume能量潮趋势".to_string(),
        category: TemplateCategory::Volume,
        source: "OBV:=SUM(IF(CLOSE>REF(CLOSE,1),VOLUME,IF(CLOSE<REF(CLOSE,1),-VOLUME,0)),N); CROSS(OBV,MA(OBV,M))".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 30.0), ("M".to_string(), 3.0, 20.0, 6.0)],
    });

    map.insert(
        "volume_ma_cross".to_string(),
        FormulaTemplate {
            name: "成交量均线交叉".to_string(),
            description: "成交量短期均线上穿长期均线".to_string(),
            category: TemplateCategory::Volume,
            source: "V5:=MA(VOLUME,5); V10:=MA(VOLUME,10); CROSS(V5,V10)".to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "volatility_volume".to_string(),
        FormulaTemplate {
            name: "成交量变异率".to_string(),
            description: "成交量的波动程度".to_string(),
            category: TemplateCategory::Volume,
            source: "MAVOL:=MA(VOLUME,N); STD(VOLUME,N)/MAVOL*100".to_string(),
            parameters: vec![("N".to_string(), 5.0, 60.0, 20.0)],
        },
    );

    // ========== 形态分析 ==========
    map.insert("double_bottom".to_string(), FormulaTemplate {
        name: "双底形态".to_string(),
        description: "W底双底形态，两个低点接近且中间有反弹".to_string(),
        category: TemplateCategory::Pattern,
        source: "L1:=LLV(LOW,N); L2:=REF(L1,N); BOUNCE:=HHV(HIGH,N/2); L1<REF(L1,1)*1.02 AND CLOSE>BOUNCE*0.98".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("double_top".to_string(), FormulaTemplate {
        name: "双顶形态".to_string(),
        description: "M头双顶形态，两个高点接近且中间有回调".to_string(),
        category: TemplateCategory::Pattern,
        source: "H1:=HHV(HIGH,N); H2:=REF(H1,N); DROP:=LLV(LOW,N/2); H1>REF(H1,1)*0.98 AND CLOSE<DROP*1.02".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("head_shoulder_top".to_string(), FormulaTemplate {
        name: "头肩顶形态".to_string(),
        description: "头肩顶反转形态，左肩头部右肩依次形成".to_string(),
        category: TemplateCategory::Pattern,
        source: "H1:=HHV(HIGH,N); L1:=LLV(LOW,N/2); H2:=HHV(HIGH,N/3); CLOSE<L1 AND H1>REF(H1,1) AND H2<H1".to_string(),
        parameters: vec![("N".to_string(), 15.0, 60.0, 30.0)],
    });

    map.insert("cup_handle".to_string(), FormulaTemplate {
        name: "杯柄形态".to_string(),
        description: "杯柄形态，圆底后小幅回调再突破".to_string(),
        category: TemplateCategory::Pattern,
        source: "HIGH_N:=HHV(HIGH,N); LOW_N:=LLV(LOW,N); MID:=(HIGH_N+LOW_N)/2; CLOSE>HIGH_N*0.95 AND LOW_N>MID*0.9".to_string(),
        parameters: vec![("N".to_string(), 20.0, 90.0, 40.0)],
    });

    map.insert("flag_pattern".to_string(), FormulaTemplate {
        name: "旗形形态".to_string(),
        description: "旗形整理形态，急涨后窄幅整理".to_string(),
        category: TemplateCategory::Pattern,
        source: "RANGE:=(HHV(HIGH,N)-LLV(LOW,N))/LLV(LOW,N)*100; RANGE<5 AND CLOSE>REF(CLOSE,N)*1.05".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 10.0)],
    });

    // ========== 趋势跟踪 ==========
    map.insert("adx_trend".to_string(), FormulaTemplate {
        name: "ADX趋势强度".to_string(),
        description: "平均方向性指数判断趋势强度".to_string(),
        category: TemplateCategory::Trend,
        source: "TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); DMP:=SUM(IF(HIGH>REF(HIGH,1) AND HIGH-REF(HIGH,1)>REF(LOW,1)-LOW,MAX(HIGH-REF(HIGH,1),HIGH-REF(HIGH,1)),0),N); DMM:=SUM(IF(LOW<REF(LOW,1) AND REF(LOW,1)-LOW>REF(HIGH,1)-HIGH,MAX(REF(LOW,1)-LOW,REF(HIGH,1)-HIGH),0),N); DI1:=DMP/TR*N*100; DI2:=DMM/TR*N*100; ADX:=MA(ABS(DI1-DI2)/(DI1+DI2)*100,M); ADX>25".to_string(),
        parameters: vec![("N".to_string(), 7.0, 21.0, 14.0), ("M".to_string(), 3.0, 12.0, 6.0)],
    });

    map.insert(
        "sar_trend".to_string(),
        FormulaTemplate {
            name: "SAR抛物线趋势".to_string(),
            description: "抛物线转向指标判断趋势方向".to_string(),
            category: TemplateCategory::Trend,
            source: "SAR=SAR(N,STEP,MAXSTEP); CLOSE>SAR".to_string(),
            parameters: vec![
                ("N".to_string(), 1.0, 10.0, 4.0),
                ("STEP".to_string(), 0.01, 0.05, 0.02),
                ("MAXSTEP".to_string(), 0.1, 0.3, 0.2),
            ],
        },
    );

    map.insert(
        "trend_strength".to_string(),
        FormulaTemplate {
            name: "趋势强度".to_string(),
            description: "基于均线斜率的趋势强度指标".to_string(),
            category: TemplateCategory::Trend,
            source: "MA20:=MA(CLOSE,20); MA5:=MA(CLOSE,5); (MA5-REF(MA5,1))/MA5*100".to_string(),
            parameters: vec![],
        },
    );

    map.insert("supertrend".to_string(), FormulaTemplate {
        name: "超级趋势".to_string(),
        description: "基于ATR的趋势跟踪指标".to_string(),
        category: TemplateCategory::Trend,
        source: "TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); MID:=MA(CLOSE,N); UPPER:=MID+ATR*MULT; LOWER:=MID-ATR*MULT; CROSS(CLOSE,LOWER)".to_string(),
        parameters: vec![("N".to_string(), 7.0, 21.0, 10.0), ("MULT".to_string(), 1.0, 4.0, 3.0)],
    });

    map.insert("ichimoku_signal".to_string(), FormulaTemplate {
        name: "一目均衡信号".to_string(),
        description: "一目均衡图转换线与基准线交叉".to_string(),
        category: TemplateCategory::Trend,
        source: "TENKAN:=(HHV(HIGH,9)+LLV(LOW,9))/2; KIJUN:=(HHV(HIGH,26)+LLV(LOW,26))/2; CROSS(TENKAN,KIJUN)".to_string(),
        parameters: vec![],
    });

    map.insert("dmi_trend".to_string(), FormulaTemplate {
        name: "DMI趋向指标".to_string(),
        description: "上升下降方向线判断趋势".to_string(),
        category: TemplateCategory::Trend,
        source: "MTR:=SUM(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),N); HD:=HIGH-REF(HIGH,1); LD:=REF(LOW,1)-LOW; DMP:=SUM(IF(HD>0 AND HD>LD,HD,0),N); DMM:=SUM(IF(LD>0 AND LD>HD,LD,0),N); PDI:=DMP/MTR*100; MDI:=DMM/MTR*100; PDI>MDI".to_string(),
        parameters: vec![("N".to_string(), 7.0, 28.0, 14.0)],
    });

    // ========== 震荡指标 ==========
    map.insert(
        "stoch_overbought".to_string(),
        FormulaTemplate {
            name: "随机指标超买".to_string(),
            description: "KD值超过80超买线".to_string(),
            category: TemplateCategory::Oscillator,
            source: "RSV:=(CLOSE-LLV(LOW,N))/(HHV(HIGH,N)-LLV(LOW,N))*100; K:=SMA(RSV,M,1); K>80"
                .to_string(),
            parameters: vec![
                ("N".to_string(), 5.0, 30.0, 14.0),
                ("M".to_string(), 2.0, 10.0, 3.0),
            ],
        },
    );

    map.insert(
        "stoch_oversold".to_string(),
        FormulaTemplate {
            name: "随机指标超卖".to_string(),
            description: "KD值低于20超卖线".to_string(),
            category: TemplateCategory::Oscillator,
            source:
                "RSV:=(CLOSE-LLV(LOW,14))/(HHV(HIGH,14)-LLV(LOW,14))*100; K:=SMA(RSV,3,1); K<20"
                    .to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "williams_r".to_string(),
        FormulaTemplate {
            name: "威廉指标".to_string(),
            description: "威廉超买超卖指标".to_string(),
            category: TemplateCategory::Oscillator,
            source: "WR:=(HHV(HIGH,N)-CLOSE)/(HHV(HIGH,N)-LLV(LOW,N))*100; WR>80".to_string(),
            parameters: vec![("N".to_string(), 5.0, 30.0, 14.0)],
        },
    );

    map.insert("cci_signal".to_string(), FormulaTemplate {
        name: "CCI顺势指标".to_string(),
        description: "CCI突破+100或-100的信号".to_string(),
        category: TemplateCategory::Oscillator,
        source: "TP:=(HIGH+LOW+CLOSE)/3; MA_TP:=MA(TP,N); MD_TP:=SUM(ABS(TP-MA_TP),N)/N; CCI:=(TP-MA_TP)/(0.015*MD_TP); CROSS(CCI,100)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 14.0)],
    });

    map.insert(
        "roc_momentum".to_string(),
        FormulaTemplate {
            name: "ROC变动率".to_string(),
            description: "价格变动率动量指标".to_string(),
            category: TemplateCategory::Oscillator,
            source: "ROC:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; ROC>0".to_string(),
            parameters: vec![("N".to_string(), 5.0, 30.0, 12.0)],
        },
    );

    map.insert(
        "momentum_signal".to_string(),
        FormulaTemplate {
            name: "动量指标".to_string(),
            description: "价格动量变化信号".to_string(),
            category: TemplateCategory::Oscillator,
            source: "MTM:=CLOSE-REF(CLOSE,N); MA_MTM:=MA(MTM,M); CROSS(MTM,MA_MTM)".to_string(),
            parameters: vec![
                ("N".to_string(), 5.0, 30.0, 12.0),
                ("M".to_string(), 3.0, 20.0, 6.0),
            ],
        },
    );

    // ========== 综合策略 ==========
    map.insert("ma_macd_strategy".to_string(), FormulaTemplate {
        name: "均线+MACD策略".to_string(),
        description: "均线多头排列且MACD金叉的综合买入策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MA5>MA10 AND MA10>MA20 AND CROSS(DIF,DEA)".to_string(),
        parameters: vec![],
    });

    map.insert("rsi_volume_strategy".to_string(), FormulaTemplate {
        name: "RSI+成交量策略".to_string(),
        description: "RSI超卖且放量反弹的买入策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MAVOL:=MA(VOLUME,5); RSI<30 AND VOLUME>MAVOL*1.5 AND CLOSE>REF(CLOSE,1)".to_string(),
        parameters: vec![],
    });

    map.insert("kdj_macd_strategy".to_string(), FormulaTemplate {
        name: "KDJ+MACD策略".to_string(),
        description: "KDJ金叉与MACD金叉共振的买入策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); CROSS(K,D) AND CROSS(DIF,DEA)".to_string(),
        parameters: vec![],
    });

    map.insert("boll_rsi_strategy".to_string(), FormulaTemplate {
        name: "布林带+RSI策略".to_string(),
        description: "触及布林下轨且RSI超卖的反弹策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MID:=MA(CLOSE,20); LOWER:=MID-STD(CLOSE,20)*2; RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; CLOSE<LOWER AND RSI<30".to_string(),
        parameters: vec![],
    });

    map.insert("ma_volume_strategy".to_string(), FormulaTemplate {
        name: "均线+成交量策略".to_string(),
        description: "均线金叉且成交量放大的确认策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MAVOL:=MA(VOLUME,5); CROSS(MA5,MA10) AND VOLUME>MAVOL*1.5".to_string(),
        parameters: vec![],
    });

    map.insert("trend_reversal".to_string(), FormulaTemplate {
        name: "趋势反转策略".to_string(),
        description: "MACD底背离加KDJ超卖的底部反转策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); J:=3*K-2*SMA(K,3,1); MACD<0 AND REF(MACD,1)<MACD AND J<20".to_string(),
        parameters: vec![],
    });

    map.insert(
        "breakout_strategy".to_string(),
        FormulaTemplate {
            name: "突破策略".to_string(),
            description: "放量突破近期高点的买入策略".to_string(),
            category: TemplateCategory::Strategy,
            source: "HIGH_N:=HHV(HIGH,N); VMA:=MA(VOLUME,M); CROSS(CLOSE,HIGH_N) AND VOLUME>VMA*2"
                .to_string(),
            parameters: vec![
                ("N".to_string(), 10.0, 60.0, 20.0),
                ("M".to_string(), 3.0, 20.0, 5.0),
            ],
        },
    );

    map.insert("golden_triangle".to_string(), FormulaTemplate {
        name: "黄金三角策略".to_string(),
        description: "5日、10日、20日均线形成黄金三角".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA5>MA10 AND MA10>MA20 AND MA5>REF(MA5,1)".to_string(),
        parameters: vec![],
    });

    map.insert("divergence_strategy".to_string(), FormulaTemplate {
        name: "背离共振策略".to_string(),
        description: "MACD与RSI同时底背离的共振买入策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MACD<0 AND REF(MACD,1)<MACD AND RSI<30 AND REF(RSI,1)<RSI".to_string(),
        parameters: vec![],
    });

    map.insert("ma_pullback".to_string(), FormulaTemplate {
        name: "均线回踩策略".to_string(),
        description: "上升趋势中回踩均线获得支撑".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); MA5:=MA(CLOSE,5); MA20>REF(MA20,1) AND CLOSE<MA5 AND CLOSE>MA20".to_string(),
        parameters: vec![],
    });

    // ========== 通达信经典 ==========
    map.insert("chip_peak".to_string(), FormulaTemplate {
        name: "筹码峰".to_string(),
        description: "基于成交量分布的筹码集中度分析".to_string(),
        category: TemplateCategory::Classic,
        source: "COST1:=WINNER(CLOSE)*100; COST2:=WINNER(CLOSE*0.9)*100; CHIP_RATIO:=(COST1-COST2)/COST1*100; CHIP_RATIO>50".to_string(),
        parameters: vec![],
    });

    map.insert("jue_lu_biao".to_string(), FormulaTemplate {
        name: "绝路航标".to_string(),
        description: "通达信经典指标，底部反转信号".to_string(),
        category: TemplateCategory::Classic,
        source: "VAR1:=LLV(LOW,21); VAR2:=HHV(HIGH,21); VAR3:=(CLOSE-VAR1)/(VAR2-VAR1)*100; VAR4:=SMA(VAR3,5,1); CROSS(VAR4,20)".to_string(),
        parameters: vec![],
    });

    map.insert(
        "dragon_head".to_string(),
        FormulaTemplate {
            name: "龙头指标".to_string(),
            description: "通达信经典龙头股识别指标".to_string(),
            category: TemplateCategory::Classic,
            source:
                "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; LTP:=VOLUME/CAPITAL*100; ZF>5 AND LTP>3"
                    .to_string(),
            parameters: vec![],
        },
    );

    map.insert(
        "main_force".to_string(),
        FormulaTemplate {
            name: "主力资金".to_string(),
            description: "主力资金流入流出指标".to_string(),
            category: TemplateCategory::Classic,
            source: "MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_NET:=SUM(MF,N); CROSS(MF_NET,0)"
                .to_string(),
            parameters: vec![("N".to_string(), 3.0, 30.0, 10.0)],
        },
    );

    map.insert("money_flow".to_string(), FormulaTemplate {
        name: "资金流向".to_string(),
        description: "大单资金净流入指标".to_string(),
        category: TemplateCategory::Classic,
        source: "BIG:=IF(VOLUME>MA(VOLUME,5)*2,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); SMALL:=IF(VOLUME<MA(VOLUME,5)*0.5,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); NET:=SUM(BIG-SMALL,N); NET>0".to_string(),
        parameters: vec![("N".to_string(), 3.0, 20.0, 5.0)],
    });

    map.insert("limit_up_capture".to_string(), FormulaTemplate {
        name: "涨停捕捉".to_string(),
        description: "捕捉即将涨停的 signals".to_string(),
        category: TemplateCategory::Classic,
        source: "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; LTP:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ZF>3 AND ZF<9 AND LTP>5 AND MA5>MA10".to_string(),
        parameters: vec![],
    });

    map.insert("bottom_fish".to_string(), FormulaTemplate {
        name: "底部吸筹".to_string(),
        description: "判断底部区域主力吸筹的指标".to_string(),
        category: TemplateCategory::Classic,
        source: "VAR1:=(CLOSE-LLV(LOW,36))/(HHV(HIGH,36)-LLV(LOW,36))*100; VAR2:=SMA(VAR1,3,1); VAR3:=SMA(VAR2,3,1); VAR4:=SMA(VAR3,3,1); CROSS(VAR4,VAR3) AND VAR4<20".to_string(),
        parameters: vec![],
    });

    map.insert("top_escape".to_string(), FormulaTemplate {
        name: "顶部逃离".to_string(),
        description: "判断顶部区域主力出货的指标".to_string(),
        category: TemplateCategory::Classic,
        source: "VAR1:=(HHV(HIGH,36)-CLOSE)/(HHV(HIGH,36)-LLV(LOW,36))*100; VAR2:=SMA(VAR1,3,1); VAR3:=SMA(VAR2,3,1); VAR4:=SMA(VAR3,3,1); CROSS(VAR3,VAR4) AND VAR3>80".to_string(),
        parameters: vec![],
    });

    map.insert("dragon_tiger".to_string(), FormulaTemplate {
        name: "龙虎榜追踪".to_string(),
        description: "追踪龙虎榜机构买卖方向".to_string(),
        category: TemplateCategory::Classic,
        source: "VAR1:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; VAR2:=VOLUME/CAPITAL*100; VAR1>5 AND VAR2>REF(VAR2,1)*2".to_string(),
        parameters: vec![],
    });

    map.insert("golden_pit".to_string(), FormulaTemplate {
        name: "黄金坑".to_string(),
        description: "深度回调后的黄金坑形态".to_string(),
        category: TemplateCategory::Classic,
        source: "VAR1:=LLV(LOW,60); VAR2:=CLOSE-VAR1; VAR3:=VAR2/VAR1*100; MA5:=MA(CLOSE,5); VAR3<20 AND CLOSE>MA5 AND REF(CLOSE,1)<REF(MA5,1)".to_string(),
        parameters: vec![],
    });

    map.insert("wave_theory".to_string(), FormulaTemplate {
        name: "波浪理论指标".to_string(),
        description: "基于波浪理论的买卖点判断".to_string(),
        category: TemplateCategory::Classic,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); MA5>MA10 AND MA10>MA20 AND MA20>MA60 AND CLOSE>MA5".to_string(),
        parameters: vec![],
    });

    map.insert("pressure_support".to_string(), FormulaTemplate {
        name: "压力支撑位".to_string(),
        description: "计算关键的压力位和支撑位".to_string(),
        category: TemplateCategory::Classic,
        source: "PP:=(HIGH+LOW+CLOSE)/3; R1:=PP*2-LOW; S1:=PP*2-HIGH; R2:=PP+(HIGH-LOW); S2:=PP-(HIGH-LOW); CLOSE>R1 OR CLOSE<S1".to_string(),
        parameters: vec![],
    });

    map.insert(
        "change_rate".to_string(),
        FormulaTemplate {
            name: "换手率指标".to_string(),
            description: "换手率异常放大的信号".to_string(),
            category: TemplateCategory::Classic,
            source: "HSL:=VOLUME/CAPITAL*100; MA_HSL:=MA(HSL,N); CROSS(HSL,MA_HSL*2)".to_string(),
            parameters: vec![("N".to_string(), 3.0, 20.0, 5.0)],
        },
    );

    map.insert("volume_price_divergence".to_string(), FormulaTemplate {
        name: "量价背离".to_string(),
        description: "价格与成交量出现背离".to_string(),
        category: TemplateCategory::Classic,
        source: "PRICE_UP:=CLOSE>REF(CLOSE,1); VOL_DOWN:=VOLUME<REF(VOLUME,1); PRICE_UP AND VOL_DOWN AND CLOSE>MA(CLOSE,20)".to_string(),
        parameters: vec![],
    });

    map.insert("gap_fill".to_string(), FormulaTemplate {
        name: "缺口回补".to_string(),
        description: "跳空缺口及其回补信号".to_string(),
        category: TemplateCategory::Classic,
        source: "GAP_UP:=LOW>REF(HIGH,1); GAP_DOWN:=HIGH<REF(LOW,1); FILLED:=LOW<=REF(HIGH,1) AND REF(LOW,1)>REF(HIGH,2); (GAP_UP OR GAP_DOWN) AND FILLED".to_string(),
        parameters: vec![],
    });

    map.insert("trend_acceleration".to_string(), FormulaTemplate {
        name: "趋势加速".to_string(),
        description: "价格上涨加速的信号".to_string(),
        category: TemplateCategory::Classic,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ACCEL:=(MA5-REF(MA5,1))-(REF(MA5,1)-REF(MA5,2)); ACCEL>0 AND CLOSE>MA5".to_string(),
        parameters: vec![],
    });

    // ========== 通达信经典指标 ==========
    map.insert("tdx_cyqkl".to_string(), FormulaTemplate {
        name: "筹码峰指标CYQKL".to_string(),
        description: "通达信筹码分布指标，分析筹码集中度和获利比例".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "CYQKL:=(WINNER(CLOSE*1.1)-WINNER(CLOSE*0.9))*100; COST90:=COST(90); COST10:=COST(10); CONCENTR:=(COST90-COST10)/(COST90+COST10)*100; CYQKL>60 AND CONCENTR<30".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_lhb".to_string(), FormulaTemplate {
        name: "龙虎榜指标LHB".to_string(),
        description: "通达信龙虎榜追踪，监控机构与游资动向".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; BIGBUY:=IF(VOLUME>MA(VOLUME,20)*3,IF(CLOSE>REF(CLOSE,1),VOLUME,0),0); BIGSELL:=IF(VOLUME>MA(VOLUME,20)*3,IF(CLOSE<REF(CLOSE,1),VOLUME,0),0); NET:=SUM(BIGBUY-BIGSELL,5); ZF>7 AND HSL>10 AND NET>0".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_ztct".to_string(), FormulaTemplate {
        name: "涨停捕捉ZTCT".to_string(),
        description: "通达信涨停板捕捉，识别即将涨停的强势股".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); STRONG:=MA5>MA10 AND MA10>MA20; VOL_UP:=VOLUME>MA(VOLUME,5)*1.5; ZF>5 AND ZF<9.5 AND STRONG AND VOL_UP AND HSL>3".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_dtct".to_string(), FormulaTemplate {
        name: "跌停捕捉DTCT".to_string(),
        description: "通达信跌停板捕捉，识别可能跌停的弱势股风险".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); WEAK:=MA5<MA10 AND MA10<MA20; VOL_UP:=VOLUME>MA(VOLUME,5)*2; ZF<-5 AND WEAK AND VOL_UP".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_qsgl".to_string(), FormulaTemplate {
        name: "强势股筛选QSGL".to_string(),
        description: "通达信强势股筛选，识别连续上涨的强势股票".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "UP_DAYS:=COUNT(CLOSE>REF(CLOSE,1),5); ZF_SUM:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,5); MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); UP_DAYS>=4 AND ZF_SUM>10 AND MA5>MA10 AND MA10>MA20 AND CLOSE>MA5".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_rsgl".to_string(), FormulaTemplate {
        name: "弱势股筛选RSGL".to_string(),
        description: "通达信弱势股筛选，识别连续下跌的弱势股票".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "DOWN_DAYS:=COUNT(CLOSE<REF(CLOSE,1),5); ZF_SUM:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,5); MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); DOWN_DAYS>=4 AND ZF_SUM<-10 AND MA5<MA10 AND MA10<MA20 AND CLOSE<MA5".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_bklh".to_string(), FormulaTemplate {
        name: "板块联动BKLH".to_string(),
        description: "通达信板块联动分析，识别板块内领涨股".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); STRONG:=MA5>MA10 AND MA10>REF(MA10,1); LEAD:=ZF>3 AND HSL>5; STRONG AND LEAD".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_zjtj".to_string(), FormulaTemplate {
        name: "主力统计ZJTJ".to_string(),
        description: "通达信主力资金统计，追踪主力买卖行为".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "VAR1:=IF(CLOSE>REF(CLOSE,1),VOLUME,IF(CLOSE<REF(CLOSE,1),-VOLUME,0)); MAIN_FORCE:=SUM(VAR1,5); MAIN_MA:=MA(MAIN_FORCE,10); CROSS(MAIN_FORCE,MAIN_MA)".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_jdcs".to_string(), FormulaTemplate {
        name: "阶段涨幅JDCS".to_string(),
        description: "通达信阶段涨幅统计，计算N日累计涨幅".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZF_N:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,N); MA_ZF:=MA((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100,5); ZF_N>M*MA_ZF".to_string(),
        parameters: vec![("N".to_string(), 5.0, 60.0, 20.0), ("M".to_string(), 1.0, 3.0, 1.5)],
    });

    map.insert("tdx_cyfp".to_string(), FormulaTemplate {
        name: "筹码分布CYFP".to_string(),
        description: "通达信筹码分布分析，计算不同价位筹码占比".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "WINNER10:=WINNER(CLOSE*0.9)*100; WINNER20:=WINNER(CLOSE*0.95)*100; WINNER50:=WINNER(CLOSE)*100; WINNER80:=WINNER(CLOSE*1.05)*100; CHIP_DENSITY:=WINNER80-WINNER10; CHIP_DENSITY>50 AND WINNER50>30".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_flzt".to_string(), FormulaTemplate {
        name: "分时涨停FLZT".to_string(),
        description: "通达信分时涨停预警，盘中实时监控涨停概率".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZF:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; HSL:=VOLUME/CAPITAL*100; VOL_RATIO:=VOLUME/MA(VOLUME,5); BUY_RATIO:=IF(CLOSE>REF(CLOSE,1),VOLUME,0)/VOLUME; ZF>6 AND HSL>5 AND VOL_RATIO>2 AND BUY_RATIO>0.6".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_jcmm".to_string(), FormulaTemplate {
        name: "进出明细JCMM".to_string(),
        description: "通达信资金进出明细，分析大单买卖方向".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "BIG_VOL:=VOLUME>MA(VOLUME,20)*2; BIG_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_VOL,VOLUME,0); BIG_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_VOL,VOLUME,0); NET_BIG:=SUM(BIG_BUY-BIG_SELL,5); NET_BIG>0".to_string(),
        parameters: vec![],
    });

    map.insert("tdx_hpdr".to_string(), FormulaTemplate {
        name: "横盘突破HPDR".to_string(),
        description: "通达信横盘突破识别，捕捉整理后的突破机会".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "HHV_N:=HHV(HIGH,N); LLV_N:=LLV(LOW,N); RANGE_PCT:=(HHV_N-LLV_N)/LLV_N*100; VOL_UP:=VOLUME>MA(VOLUME,5)*1.5; BREAK_OUT:=CLOSE>HHV_N*0.98; RANGE_PCT<15 AND VOL_UP AND BREAK_OUT".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("tdx_zlcp".to_string(), FormulaTemplate {
        name: "主力成本ZLCP".to_string(),
        description: "通达信主力成本分析，计算主力平均持仓成本".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "AVG_COST:=SUM(AMOUNT,N)/SUM(VOLUME,N); COST_DIFF:=(CLOSE-AVG_COST)/AVG_COST*100; COST_DIFF>-5 AND COST_DIFF<5 AND VOLUME>MA(VOLUME,5)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("tdx_sxbd".to_string(), FormulaTemplate {
        name: "双响炮SXD".to_string(),
        description: "通达信双响炮形态，连续涨停后的回调再启动".to_string(),
        category: TemplateCategory::TDXClassic,
        source: "ZT1:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100-9.9)<0.5; ZT_DAYS:=COUNT(ZT1,10); ADJ_DAYS:=COUNT(CLOSE<REF(CLOSE,1),3); MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); ZT_DAYS>=2 AND ADJ_DAYS<=2 AND CROSS(MA5,MA10)".to_string(),
        parameters: vec![],
    });

    // ========== 同花顺智能选股 ==========
    map.insert("ths_macd_select".to_string(), FormulaTemplate {
        name: "MACD金叉选股".to_string(),
        description: "同花顺MACD金叉智能选股，DIF上穿DEA且在零轴上方".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD:=(DIF-DEA)*2; ZERO_ABOVE:=DIF>0 AND DEA>0; GOLDEN_CROSS:=CROSS(DIF,DEA); ZERO_ABOVE AND GOLDEN_CROSS".to_string(),
        parameters: vec![],
    });

    map.insert("ths_kdj_oversold".to_string(), FormulaTemplate {
        name: "KDJ超卖选股".to_string(),
        description: "同花顺KDJ超卖选股，J值低于20且出现金叉".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "RSV:=(CLOSE-LLV(LOW,9))/(HHV(HIGH,9)-LLV(LOW,9))*100; K:=SMA(RSV,3,1); D:=SMA(K,3,1); J:=3*K-2*D; J<20 AND CROSS(K,D)".to_string(),
        parameters: vec![],
    });

    map.insert("ths_ma_bullish".to_string(), FormulaTemplate {
        name: "均线多头排列选股".to_string(),
        description: "同花顺均线多头排列选股，短中长期均线依次排列".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); MA120:=MA(CLOSE,120); BULLISH:=MA5>MA10 AND MA10>MA20 AND MA20>MA60; TREND_UP:=MA5>REF(MA5,1) AND MA10>REF(MA10,1); BULLISH AND TREND_UP AND CLOSE>MA5".to_string(),
        parameters: vec![],
    });

    map.insert("ths_vol_price".to_string(), FormulaTemplate {
        name: "量价齐升选股".to_string(),
        description: "同花顺量价齐升选股，价涨量增确认趋势".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "PRICE_UP:=CLOSE>REF(CLOSE,1) AND REF(CLOSE,1)>REF(CLOSE,2); VOL_UP:=VOLUME>REF(VOLUME,1) AND REF(VOLUME,1)>REF(VOLUME,2); MA5_UP:=MA(CLOSE,5)>REF(MA(CLOSE,5),1); MA10_UP:=MA(CLOSE,10)>REF(MA(CLOSE,10),1); PRICE_UP AND VOL_UP AND MA5_UP AND MA10_UP".to_string(),
        parameters: vec![],
    });

    map.insert("ths_breakout".to_string(), FormulaTemplate {
        name: "突破选股".to_string(),
        description: "同花顺突破选股，放量突破前期高点".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "HHV_N:=HHV(HIGH,N); VOL_MA:=MA(VOLUME,M); BREAK_PRICE:=CLOSE>REF(HHV_N,1); BREAK_VOL:=VOLUME>REF(VOL_MA,1)*1.5; TREND_UP:=MA(CLOSE,5)>MA(CLOSE,10); BREAK_PRICE AND BREAK_VOL AND TREND_UP".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0), ("M".to_string(), 3.0, 20.0, 5.0)],
    });

    map.insert("ths_rsi_rebound".to_string(), FormulaTemplate {
        name: "RSI反弹选股".to_string(),
        description: "同花顺RSI超卖反弹选股，RSI从超卖区回升".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; RSI_LOW:=REF(RSI,1)<30; RSI_UP:=RSI>REF(RSI,1); VOL_UP:=VOLUME>MA(VOLUME,5); RSI_LOW AND RSI_UP AND VOL_UP".to_string(),
        parameters: vec![],
    });

    map.insert("ths_boll_support".to_string(), FormulaTemplate {
        name: "布林支撑选股".to_string(),
        description: "同花顺布林带下轨支撑选股，触及下轨后反弹".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "MID:=MA(CLOSE,20); UPPER:=MID+STD(CLOSE,20)*2; LOWER:=MID-STD(CLOSE,20)*2; TOUCH_LOWER:=REF(LOW,1)<LOWER OR REF(LOW,2)<LOWER; REBOUND:=CLOSE>REF(CLOSE,1); VOL_UP:=VOLUME>MA(VOLUME,5); TOUCH_LOWER AND REBOUND AND VOL_UP".to_string(),
        parameters: vec![],
    });

    map.insert("ths_golden_cross".to_string(), FormulaTemplate {
        name: "三线金叉选股".to_string(),
        description: "同花顺三线金叉选股，均线、成交量、MACD同时金叉".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA_CROSS:=CROSS(MA5,MA10); V5:=MA(VOLUME,5); V10:=MA(VOLUME,10); VOL_CROSS:=CROSS(V5,V10); DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); MACD_CROSS:=CROSS(DIF,DEA); MA_CROSS AND VOL_CROSS AND MACD_CROSS".to_string(),
        parameters: vec![],
    });

    map.insert("ths_strong_pullback".to_string(), FormulaTemplate {
        name: "强势回踩选股".to_string(),
        description: "同花顺强势股回踩选股，强势股回调至支撑位".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); STRONG:=MA5>MA10 AND MA10>MA20 AND MA20>REF(MA20,1); PULLBACK:=CLOSE<MA5 AND CLOSE>MA10; VOL_SHRINK:=VOLUME<MA(VOLUME,5); STRONG AND PULLBACK AND VOL_SHRINK".to_string(),
        parameters: vec![],
    });

    map.insert("ths_volume_break".to_string(), FormulaTemplate {
        name: "放量突破选股".to_string(),
        description: "同花顺放量突破选股，成交量显著放大突破".to_string(),
        category: TemplateCategory::THSSmartSelect,
        source: "VOL_MA5:=MA(VOLUME,5); VOL_MA10:=MA(VOLUME,10); VOL_RATIO:=VOLUME/VOL_MA5; PRICE_BREAK:=CLOSE>HHV(HIGH,20); VOL_BREAK:=VOL_RATIO>2 AND VOLUME>VOL_MA10; TREND_UP:=MA(CLOSE,5)>MA(CLOSE,10); PRICE_BREAK AND VOL_BREAK AND TREND_UP".to_string(),
        parameters: vec![],
    });

    // ========== 大智慧资金流向 ==========
    map.insert("dzh_main_inflow".to_string(), FormulaTemplate {
        name: "主力资金流入".to_string(),
        description: "大智慧主力资金流入监控，追踪大资金动向".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "BIG_VOL:=VOLUME>MA(VOLUME,20)*1.5; MAIN_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_VOL,VOLUME*CLOSE,0); MAIN_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_VOL,VOLUME*CLOSE,0); NET_FLOW:=SUM(MAIN_BUY-MAIN_SELL,N); NET_FLOW>0 AND REF(NET_FLOW,1)<0".to_string(),
        parameters: vec![("N".to_string(), 3.0, 20.0, 5.0)],
    });

    map.insert("dzh_big_order_net".to_string(), FormulaTemplate {
        name: "大单净流入".to_string(),
        description: "大智慧大单净流入统计，分析超大单买卖方向".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "AVG_PRICE:=AMOUNT/VOLUME; BIG_ORDER:=VOLUME>MA(VOLUME,20)*2; SUPER_BIG:=VOLUME>MA(VOLUME,20)*5; BIG_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_ORDER,VOLUME,0); BIG_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_ORDER,VOLUME,0); NET_BIG:=SUM(BIG_BUY-BIG_SELL,3); NET_BIG>MA(VOLUME,20)".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_flow_trend".to_string(), FormulaTemplate {
        name: "资金流向趋势".to_string(),
        description: "大智慧资金流向趋势分析，判断资金持续流入流出".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_MA5:=MA(MF,5); MF_MA10:=MA(MF,10); MF_MA20:=MA(MF,20); TREND_UP:=MF_MA5>MF_MA10 AND MF_MA10>MF_MA20; FLOW_POS:=SUM(MF,5)>0; TREND_UP AND FLOW_POS".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_sector_flow".to_string(), FormulaTemplate {
        name: "板块资金流向".to_string(),
        description: "大智慧板块资金流向分析，识别板块资金动向".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "SECTOR_VOL:=SUM(VOLUME,N); SECTOR_AMOUNT:=SUM(AMOUNT,N); AVG_PRICE:=SECTOR_AMOUNT/SECTOR_VOL; INDIV_FLOW:=IF(CLOSE>AVG_PRICE,VOLUME,-VOLUME); NET_FLOW:=SUM(INDIV_FLOW,5); MA_FLOW:=MA(NET_FLOW,10); CROSS(NET_FLOW,MA_FLOW)".to_string(),
        parameters: vec![("N".to_string(), 3.0, 20.0, 5.0)],
    });

    map.insert("dzh_smart_money".to_string(), FormulaTemplate {
        name: "聪明资金追踪".to_string(),
        description: "大智慧聪明资金追踪，识别机构资金动向".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "SMART_BUY:=IF(CLOSE>REF(CLOSE,1) AND VOLUME>MA(VOLUME,20)*2,VOLUME*CLOSE,0); SMART_SELL:=IF(CLOSE<REF(CLOSE,1) AND VOLUME>MA(VOLUME,20)*2,VOLUME*CLOSE,0); NET_SMART:=SUM(SMART_BUY-SMART_SELL,10); SMA_NET:=MA(NET_SMART,5); NET_SMART>SMA_NET AND NET_SMART>0".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_retail_main".to_string(), FormulaTemplate {
        name: "散户主力对比".to_string(),
        description: "大智慧散户与主力资金对比分析".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "MAIN_VOL:=IF(VOLUME>MA(VOLUME,20)*1.5,VOLUME,0); RETAIL_VOL:=IF(VOLUME<MA(VOLUME,20)*0.8,VOLUME,0); MAIN_NET:=SUM(IF(CLOSE>REF(CLOSE,1),MAIN_VOL,-MAIN_VOL),5); RETAIL_NET:=SUM(IF(CLOSE>REF(CLOSE,1),RETAIL_VOL,-RETAIL_VOL),5); MAIN_NET>0 AND RETAIL_NET<0".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_continuous_inflow".to_string(), FormulaTemplate {
        name: "连续资金流入".to_string(),
        description: "大智慧连续资金流入统计，识别持续流入股票".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "DAY_FLOW:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); POS_DAYS:=COUNT(DAY_FLOW>0,N); CONSEC_UP:=POS_DAYS>=M; MA_FLOW:=MA(DAY_FLOW,5); FLOW_TREND:=DAY_FLOW>MA_FLOW; CONSEC_UP AND FLOW_TREND".to_string(),
        parameters: vec![("N".to_string(), 3.0, 10.0, 5.0), ("M".to_string(), 2.0, 5.0, 3.0)],
    });

    map.insert("dzh_turnover_rate".to_string(), FormulaTemplate {
        name: "换手率资金流".to_string(),
        description: "大智慧换手率与资金流结合分析".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "HSL:=VOLUME/CAPITAL*100; MA_HSL:=MA(HSL,5); HIGH_HSL:=HSL>MA_HSL*1.5; PRICE_UP:=CLOSE>REF(CLOSE,1); MONEY_IN:=IF(PRICE_UP AND HIGH_HSL,AMOUNT,0); NET_IN:=SUM(MONEY_IN,5); NET_IN>MA(AMOUNT,5)*1.2".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_top_bottom".to_string(), FormulaTemplate {
        name: "资金顶底判断".to_string(),
        description: "大智慧资金流向判断顶底，资金背离分析".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "MF:=IF(CLOSE>REF(CLOSE,1),VOLUME,-VOLUME); MF_MA:=MA(MF,10); PRICE_UP:=CLOSE>REF(CLOSE,1); FLOW_DOWN:=MF<MF_MA; TOP_DIVERGE:=PRICE_UP AND FLOW_DOWN; PRICE_DOWN:=CLOSE<REF(CLOSE,1); FLOW_UP:=MF>MF_MA; BOTTOM_DIVERGE:=PRICE_DOWN AND FLOW_UP; TOP_DIVERGE OR BOTTOM_DIVERGE".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_accumulation".to_string(), FormulaTemplate {
        name: "主力吸筹监控".to_string(),
        description: "大智慧主力吸筹行为监控，识别底部吸筹".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "LOW_PRICE:=CLOSE<MA(CLOSE,60); VOL_SHRINK:=VOLUME<MA(VOLUME,20)*0.7; SMALL_UP:=IF(CLOSE>REF(CLOSE,1) AND VOLUME<MA(VOLUME,20),VOLUME,0); ACCUM:=SUM(SMALL_UP,10); ACCUM_TREND:=ACCUM>REF(ACCUM,5); LOW_PRICE AND ACCUM_TREND".to_string(),
        parameters: vec![],
    });

    map.insert("dzh_distribution".to_string(), FormulaTemplate {
        name: "主力出货监控".to_string(),
        description: "大智慧主力出货行为监控，识别高位出货".to_string(),
        category: TemplateCategory::DZHMoneyFlow,
        source: "HIGH_PRICE:=CLOSE>MA(CLOSE,60)*1.2; VOL_EXPAND:=VOLUME>MA(VOLUME,20)*1.5; BIG_DOWN:=IF(CLOSE<REF(CLOSE,1) AND VOLUME>MA(VOLUME,20),VOLUME,0); DISTRIB:=SUM(BIG_DOWN,5); DISTRIB_TREND:=DISTRIB>REF(DISTRIB,3); HIGH_PRICE AND DISTRIB_TREND".to_string(),
        parameters: vec![],
    });

    // ========== 东方财富(EM)经典指标 ==========
    map.insert("em_bull_bear".to_string(), FormulaTemplate {
        name: "多空博弈".to_string(),
        description: "东方财富多空博弈指标，基于买卖力量对比判断多空趋势".to_string(),
        category: TemplateCategory::EMClassic,
        source: "BUYV:=IF(CLOSE>REF(CLOSE,1),VOLUME,0); SELLV:=IF(CLOSE<REF(CLOSE,1),VOLUME,0); NET_BUY:=BUYV-SELLV; MA_NET:=MA(NET_BUY,5); EM_CROSS(NET_BUY,MA_NET)".to_string(),
        parameters: vec![],
    });

    map.insert("em_fund_trend".to_string(), FormulaTemplate {
        name: "资金趋势".to_string(),
        description: "东方财富资金趋势指标，追踪主力资金流入流出方向".to_string(),
        category: TemplateCategory::EMClassic,
        source: "MF:=IF(CLOSE>REF(CLOSE,1),AMOUNT,-AMOUNT); MF_MA5:=MA(MF,5); MF_MA10:=MA(MF,10); MF_MA20:=MA(MF,20); TREND_UP:=MF_MA5>MF_MA10 AND MF_MA10>MF_MA20; TREND_UP AND MF>0".to_string(),
        parameters: vec![],
    });

    map.insert("em_main_track".to_string(), FormulaTemplate {
        name: "主力追踪".to_string(),
        description: "东方财富主力追踪指标，监控主力资金动向".to_string(),
        category: TemplateCategory::EMClassic,
        source: "BIG_VOL:=VOLUME>MA(VOLUME,20)*1.5; MAIN_BUY:=IF(CLOSE>REF(CLOSE,1) AND BIG_VOL,AMOUNT,0); MAIN_SELL:=IF(CLOSE<REF(CLOSE,1) AND BIG_VOL,AMOUNT,0); NET_MAIN:=SUM(MAIN_BUY-MAIN_SELL,5); MA_MAIN:=MA(NET_MAIN,10); EM_CROSS(NET_MAIN,MA_MAIN)".to_string(),
        parameters: vec![],
    });

    map.insert("em_cost_dist".to_string(), FormulaTemplate {
        name: "成本分布".to_string(),
        description: "东方财富成本分布指标，计算加权平均成本价".to_string(),
        category: TemplateCategory::EMClassic,
        source: "AVG_COST:=EM_COSTEX(CLOSE,VOLUME); MA_COST:=MA(AVG_COST,10); DEV:=(CLOSE-AVG_COST)/AVG_COST*100; DEV<-5 AND CLOSE>MA_COST".to_string(),
        parameters: vec![],
    });

    map.insert("em_smart_select".to_string(), FormulaTemplate {
        name: "智能选股EM版".to_string(),
        description: "东方财富智能选股，综合多空、资金、趋势信号".to_string(),
        category: TemplateCategory::EMClassic,
        source: "BUYV:=IF(CLOSE>REF(CLOSE,1),VOLUME,0); SELLV:=IF(CLOSE<REF(CLOSE,1),VOLUME,0); NET_BUY:=BUYV-SELLV; MA_NET:=MA(NET_BUY,5); BULL_SIGNAL:=EM_CROSS(NET_BUY,MA_NET); MF:=IF(CLOSE>REF(CLOSE,1),AMOUNT,-AMOUNT); MF_MA:=MA(MF,10); FUND_SIGNAL:=MF>MF_MA; TREND:=MA(CLOSE,5)>MA(CLOSE,20); BULL_SIGNAL AND FUND_SIGNAL AND TREND".to_string(),
        parameters: vec![],
    });

    map.insert("em_bull_bear_strength".to_string(), FormulaTemplate {
        name: "多空强度".to_string(),
        description: "多空列强度对比".to_string(),
        category: TemplateCategory::EMClassic,
        source: "DKCOL/MA(VOLUME,5)".to_string(),
        parameters: vec![],
    });

    map.insert("em_fund_inflow".to_string(), FormulaTemplate {
        name: "资金流入".to_string(),
        description: "主力资金流入判断".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_ZLCCV()".to_string(),
        parameters: vec![],
    });

    map.insert("em_cost_support".to_string(), FormulaTemplate {
        name: "成本支撑".to_string(),
        description: "成本价支撑位".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_COSTEX(CLOSE,VOLUME)".to_string(),
        parameters: vec![],
    });

    map.insert("em_cross_golden".to_string(), FormulaTemplate {
        name: "EM金叉".to_string(),
        description: "EM版金叉信号".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_CROSS(MA(CLOSE,5),MA(CLOSE,20))".to_string(),
        parameters: vec![],
    });

    map.insert("em_cross_dead".to_string(), FormulaTemplate {
        name: "EM死叉".to_string(),
        description: "EM版死叉信号".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_CROSS(MA(CLOSE,20),MA(CLOSE,5))".to_string(),
        parameters: vec![],
    });

    map.insert("em_zig_trend".to_string(), FormulaTemplate {
        name: "之字趋势".to_string(),
        description: "EM之字转向趋势".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_ZIG(1,5)".to_string(),
        parameters: vec![],
    });

    map.insert("em_peak_resistance".to_string(), FormulaTemplate {
        name: "峰值阻力".to_string(),
        description: "之字峰值阻力位".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_PEAK(1,5,1)".to_string(),
        parameters: vec![],
    });

    map.insert("em_trough_support".to_string(), FormulaTemplate {
        name: "谷值支撑".to_string(),
        description: "之字谷值支撑位".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_TROUGH(1,5,1)".to_string(),
        parameters: vec![],
    });

    map.insert("em_smart_combo".to_string(), FormulaTemplate {
        name: "智能综合".to_string(),
        description: "多信号综合".to_string(),
        category: TemplateCategory::EMClassic,
        source: "DKCOL;EM_CROSS(MA(C,5),MA(C,20));EM_ZLCCV()".to_string(),
        parameters: vec![],
    });

    map.insert("em_volume_price".to_string(), FormulaTemplate {
        name: "量价分析".to_string(),
        description: "量价配合分析".to_string(),
        category: TemplateCategory::EMClassic,
        source: "EM_COSTEX(CLOSE,VOLUME);DKCOL".to_string(),
        parameters: vec![],
    });

    // ========== 额外经典指标补充 ==========
    map.insert("expma_cross".to_string(), FormulaTemplate {
        name: "EXPMA交叉".to_string(),
        description: "指数平均线交叉信号，反应更快".to_string(),
        category: TemplateCategory::MovingAverage,
        source: "EXP1:=EMA(CLOSE,SHORT); EXP2:=EMA(CLOSE,LONG); CROSS(EXP1,EXP2)".to_string(),
        parameters: vec![("SHORT".to_string(), 5.0, 30.0, 12.0), ("LONG".to_string(), 10.0, 60.0, 50.0)],
    });

    map.insert("vwap_deviation".to_string(), FormulaTemplate {
        name: "VWAP偏离".to_string(),
        description: "成交量加权平均价偏离度".to_string(),
        category: TemplateCategory::MovingAverage,
        source: "VWAP:=SUM(AMOUNT,N)/SUM(VOLUME,N); DEV:=(CLOSE-VWAP)/VWAP*100; DEV>THRESHOLD".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 20.0), ("THRESHOLD".to_string(), 1.0, 5.0, 2.0)],
    });

    map.insert("trix_signal".to_string(), FormulaTemplate {
        name: "TRIX指标".to_string(),
        description: "三重指数平滑移动平均指标".to_string(),
        category: TemplateCategory::Oscillator,
        source: "TR:=EMA(EMA(EMA(CLOSE,N),N),N); TRIX:=(TR-REF(TR,1))/REF(TR,1)*100; TRMA:=MA(TRIX,M); CROSS(TRIX,TRMA)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 12.0), ("M".to_string(), 5.0, 20.0, 9.0)],
    });

    map.insert("dpo_oscillator".to_string(), FormulaTemplate {
        name: "DPO去趋势".to_string(),
        description: "去趋势价格震荡指标，消除趋势影响".to_string(),
        category: TemplateCategory::Oscillator,
        source: "DPO:=CLOSE-REF(MA(CLOSE,N),N/2+1); MA_DPO:=MA(DPO,M); CROSS(DPO,MA_DPO)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0), ("M".to_string(), 3.0, 10.0, 5.0)],
    });

    map.insert("mfi_money_flow".to_string(), FormulaTemplate {
        name: "MFI资金流量".to_string(),
        description: "资金流量指标，结合价格和成交量".to_string(),
        category: TemplateCategory::Volume,
        source: "TP:=(HIGH+LOW+CLOSE)/3; MF:=TP*VOLUME; PMF:=SUM(IF(TP>REF(TP,1),MF,0),N); NMF:=SUM(IF(TP<REF(TP,1),MF,0),N); MFI:=PMF/(PMF+NMF)*100; MFI<20".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 14.0)],
    });

    map.insert("vpt_trend".to_string(), FormulaTemplate {
        name: "VPT量价趋势".to_string(),
        description: "量价趋势指标，累积成交量变化".to_string(),
        category: TemplateCategory::Volume,
        source: "VPT:=SUM((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*VOLUME,N); MA_VPT:=MA(VPT,M); CROSS(VPT,MA_VPT)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 30.0), ("M".to_string(), 3.0, 20.0, 6.0)],
    });

    map.insert("asi_accumulation".to_string(), FormulaTemplate {
        name: "ASI累积震荡".to_string(),
        description: "累积摆动指标，真实波动幅度".to_string(),
        category: TemplateCategory::Oscillator,
        source: "SI:=(CLOSE-REF(CLOSE,1)+REF(CLOSE,1)-REF(CLOSE,2))/2; ASI:=SUM(SI,N); MA_ASI:=MA(ASI,M); CROSS(ASI,MA_ASI)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0), ("M".to_string(), 3.0, 10.0, 5.0)],
    });

    map.insert("wvad_volume".to_string(), FormulaTemplate {
        name: "WVAD威廉变异".to_string(),
        description: "威廉变异离散量，量价分析".to_string(),
        category: TemplateCategory::Volume,
        source: "WVAD:=SUM((CLOSE-OPEN)/(HIGH-LOW)*VOLUME,N); MA_WVAD:=MA(WVAD,M); CROSS(WVAD,MA_WVAD)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 24.0), ("M".to_string(), 3.0, 15.0, 6.0)],
    });

    map.insert("boll_width".to_string(), FormulaTemplate {
        name: "布林带宽度".to_string(),
        description: "布林带宽度变化，判断波动性".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MID:=MA(CLOSE,N); UPPER:=MID+STD(CLOSE,N)*2; LOWER:=MID-STD(CLOSE,N)*2; WIDTH:=(UPPER-LOWER)/MID*100; WIDTH<REF(WIDTH,1)*0.7".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("env_envelope".to_string(), FormulaTemplate {
        name: "ENV包络线".to_string(),
        description: "价格包络线指标，判断超买超卖".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MID:=MA(CLOSE,N); UPPER:=MID*(1+PCT/100); LOWER:=MID*(1-PCT/100); CLOSE<LOWER".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 14.0), ("PCT".to_string(), 1.0, 10.0, 3.0)],
    });

    map.insert("mcst_cost".to_string(), FormulaTemplate {
        name: "MCST成本".to_string(),
        description: "市场成本指标，反映平均持仓成本".to_string(),
        category: TemplateCategory::Classic,
        source: "MCST:=DMA(AMOUNT/VOLUME,VOLUME/CAPITAL); COST_DIFF:=(CLOSE-MCST)/MCST*100; COST_DIFF>-5 AND COST_DIFF<5".to_string(),
        parameters: vec![],
    });

    map.insert("psy_psychological".to_string(), FormulaTemplate {
        name: "PSY心理线".to_string(),
        description: "心理线指标，反映投资者心理预期".to_string(),
        category: TemplateCategory::Oscillator,
        source: "UP_DAYS:=COUNT(CLOSE>REF(CLOSE,1),N); PSY:=UP_DAYS/N*100; PSY<25".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 12.0)],
    });

    map.insert("vr_volume_ratio".to_string(), FormulaTemplate {
        name: "VR容量比率".to_string(),
        description: "容量比率指标，量价关系分析".to_string(),
        category: TemplateCategory::Volume,
        source: "TH:=SUM(IF(CLOSE>REF(CLOSE,1),VOLUME,0),N); TL:=SUM(IF(CLOSE<REF(CLOSE,1),VOLUME,0),N); TQ:=SUM(IF(CLOSE=REF(CLOSE,1),VOLUME,0),N); VR:=(TH+TQ/2)/(TL+TQ/2)*100; VR<70".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 26.0)],
    });

    map.insert("brar_emotion".to_string(), FormulaTemplate {
        name: "BRAR情绪指标".to_string(),
        description: "买卖意愿指标，市场情绪分析".to_string(),
        category: TemplateCategory::Oscillator,
        source: "AR:=SUM(HIGH-OPEN,N)/SUM(OPEN-LOW,N)*100; BR:=SUM(MAX(HIGH-REF(CLOSE,1),0),N)/SUM(MAX(REF(CLOSE,1)-LOW,0),N)*100; AR<50 AND BR<40".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 26.0)],
    });

    map.insert("dma_difference".to_string(), FormulaTemplate {
        name: "DMA平行线差".to_string(),
        description: "平行线差指标，中长期趋势判断".to_string(),
        category: TemplateCategory::Trend,
        source: "DIF:=MA(CLOSE,SHORT)-MA(CLOSE,LONG); AMA:=MA(DIF,M); CROSS(DIF,AMA)".to_string(),
        parameters: vec![("SHORT".to_string(), 5.0, 20.0, 10.0), ("LONG".to_string(), 20.0, 60.0, 50.0), ("M".to_string(), 3.0, 10.0, 6.0)],
    });

    map.insert("bbi_multi_ma".to_string(), FormulaTemplate {
        name: "BBI多空指标".to_string(),
        description: "多空指数，多条均线综合判断".to_string(),
        category: TemplateCategory::MovingAverage,
        source: "BBI:=(MA(CLOSE,3)+MA(CLOSE,6)+MA(CLOSE,12)+MA(CLOSE,24))/4; CROSS(CLOSE,BBI)".to_string(),
        parameters: vec![],
    });

    map.insert("expmi_exponential".to_string(), FormulaTemplate {
        name: "EMI指数移动".to_string(),
        description: "指数移动创新指标".to_string(),
        category: TemplateCategory::MovingAverage,
        source: "EMI:=EMA(CLOSE,N)-EMA(EMA(CLOSE,N),N); SIGNAL:=EMA(EMI,M); CROSS(EMI,SIGNAL)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 13.0), ("M".to_string(), 3.0, 15.0, 5.0)],
    });

    map.insert("mass_index".to_string(), FormulaTemplate {
        name: "MIK质量指数".to_string(),
        description: "质量指数，识别趋势反转".to_string(),
        category: TemplateCategory::Oscillator,
        source: "RANGE:=HIGH-LOW; ER:=EMA(RANGE,N)/EMA(EMA(RANGE,N),N); MIK:=SUM(ER,M); MIK>27".to_string(),
        parameters: vec![("N".to_string(), 5.0, 15.0, 9.0), ("M".to_string(), 15.0, 35.0, 25.0)],
    });

    map.insert("kama_kaufman".to_string(), FormulaTemplate {
        name: "KAMA考夫曼均线".to_string(),
        description: "考夫曼自适应移动平均线".to_string(),
        category: TemplateCategory::MovingAverage,
        source: "DIRECTION:=ABS(CLOSE-REF(CLOSE,N)); VOLATILITY:=SUM(ABS(CLOSE-REF(CLOSE,1)),N); ER:=DIRECTION/VOLATILITY; FAST:=2/(FAST_P+1); SLOW:=2/(SLOW_P+1); SC:=ER*(FAST-SLOW)+SLOW; KAMA:=REF(CLOSE,1)+SC*(CLOSE-REF(CLOSE,1)); CROSS(CLOSE,KAMA)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0), ("FAST_P".to_string(), 2.0, 10.0, 2.0), ("SLOW_P".to_string(), 20.0, 40.0, 30.0)],
    });

    map.insert("vortex_indicator".to_string(), FormulaTemplate {
        name: "VI涡旋指标".to_string(),
        description: "涡旋指标，识别趋势方向".to_string(),
        category: TemplateCategory::Trend,
        source: "VM_PLUS:=ABS(HIGH-REF(LOW,1)); VM_MINUS:=ABS(LOW-REF(HIGH,1)); TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); VI_PLUS:=SUM(VM_PLUS,N)/SUM(TR,N); VI_MINUS:=SUM(VM_MINUS,N)/SUM(TR,N); CROSS(VI_PLUS,VI_MINUS)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 14.0)],
    });

    map.insert("kst_momentum".to_string(), FormulaTemplate {
        name: "KST确知指标".to_string(),
        description: "确知指标，综合动量分析".to_string(),
        category: TemplateCategory::Oscillator,
        source: "ROC1:=MA((CLOSE-REF(CLOSE,10))/REF(CLOSE,10)*100,10); ROC2:=MA((CLOSE-REF(CLOSE,15))/REF(CLOSE,15)*100,10); ROC3:=MA((CLOSE-REF(CLOSE,20))/REF(CLOSE,20)*100,10); ROC4:=MA((CLOSE-REF(CLOSE,30))/REF(CLOSE,30)*100,15); KST:=ROC1+ROC2*2+ROC3*3+ROC4*4; SIGNAL:=MA(KST,9); CROSS(KST,SIGNAL)".to_string(),
        parameters: vec![],
    });

    map.insert("ppo_percentage".to_string(), FormulaTemplate {
        name: "PPO百分比震荡".to_string(),
        description: "价格震荡百分比指标".to_string(),
        category: TemplateCategory::Oscillator,
        source: "PPO:=(EMA(CLOSE,SHORT)-EMA(CLOSE,LONG))/EMA(CLOSE,LONG)*100; SIGNAL:=EMA(PPO,M); CROSS(PPO,SIGNAL)".to_string(),
        parameters: vec![("SHORT".to_string(), 5.0, 15.0, 12.0), ("LONG".to_string(), 20.0, 35.0, 26.0), ("M".to_string(), 5.0, 15.0, 9.0)],
    });

    map.insert("donchian_breakout".to_string(), FormulaTemplate {
        name: "唐奇安通道突破".to_string(),
        description: "唐奇安通道突破策略".to_string(),
        category: TemplateCategory::Trend,
        source: "UPPER:=HHV(HIGH,N); LOWER:=LLV(LOW,N); MID:=(UPPER+LOWER)/2; CROSS(CLOSE,UPPER)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("keltner_channel".to_string(), FormulaTemplate {
        name: "肯特纳通道".to_string(),
        description: "肯特纳通道，基于ATR的波动通道".to_string(),
        category: TemplateCategory::Oscillator,
        source: "TYP:=(HIGH+LOW+CLOSE)/3; ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),N); UPPER:=MA(TYP,N)+ATR*M; LOWER:=MA(TYP,N)-ATR*M; CLOSE<LOWER".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0), ("M".to_string(), 1.0, 3.0, 2.0)],
    });

    map.insert("zigzag_trend".to_string(), FormulaTemplate {
        name: "之字转向".to_string(),
        description: "之字转向指标，识别趋势转折点".to_string(),
        category: TemplateCategory::Trend,
        source: "HH:=HHV(HIGH,N); LL:=LLV(LOW,N); PCT_CHANGE:=(HH-LL)/LL*100; TREND_UP:=CLOSE>REF(HH,1)*0.95; PCT_CHANGE>THRESHOLD AND TREND_UP".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0), ("THRESHOLD".to_string(), 3.0, 10.0, 5.0)],
    });

    map.insert("adl_accumulation".to_string(), FormulaTemplate {
        name: "ADL累积派发".to_string(),
        description: "累积派发线，资金流向分析".to_string(),
        category: TemplateCategory::Volume,
        source: "MFM:=((CLOSE-LOW)-(HIGH-CLOSE))/(HIGH-LOW); MFV:=MFM*VOLUME; ADL:=SUM(MFV,N); MA_ADL:=MA(ADL,M); CROSS(ADL,MA_ADL)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 14.0), ("M".to_string(), 3.0, 15.0, 5.0)],
    });

    map.insert("cmf_chaikin".to_string(), FormulaTemplate {
        name: "CMF佳庆资金流".to_string(),
        description: "佳庆资金流量指标".to_string(),
        category: TemplateCategory::Volume,
        source: "MFM:=((CLOSE-LOW)-(HIGH-CLOSE))/(HIGH-LOW); MFV:=MFM*VOLUME; CMF:=SUM(MFV,N)/SUM(VOLUME,N); CMF>0".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("evm_ease".to_string(), FormulaTemplate {
        name: "EVM简易波动".to_string(),
        description: "简易波动指标，量价关系分析".to_string(),
        category: TemplateCategory::Volume,
        source: "DM:=((HIGH+LOW)/2-(REF(HIGH,1)+REF(LOW,1))/2); BR:=(HIGH-LOW); EVM:=DM/BR*VOLUME; MA_EVM:=MA(EVM,N); CROSS(EVM,MA_EVM)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 14.0)],
    });

    map.insert("fi_force".to_string(), FormulaTemplate {
        name: "FI力度指数".to_string(),
        description: "力度指数，价格与成交量综合".to_string(),
        category: TemplateCategory::Volume,
        source: "FI:=(CLOSE-REF(CLOSE,1))*VOLUME; MA_FI:=MA(FI,N); CROSS(FI,MA_FI)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 13.0)],
    });

    map.insert("nvi_negative".to_string(), FormulaTemplate {
        name: "NVI负量指标".to_string(),
        description: "负量指标，缩量日价格变化".to_string(),
        category: TemplateCategory::Volume,
        source: "NVI:=IF(VOLUME<REF(VOLUME,1),REF(NVI,1)+(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*REF(NVI,1),REF(NVI,1)); MA_NVI:=MA(NVI,M); CROSS(NVI,MA_NVI)".to_string(),
        parameters: vec![("M".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("pvi_positive".to_string(), FormulaTemplate {
        name: "PVI正量指标".to_string(),
        description: "正量指标，放量日价格变化".to_string(),
        category: TemplateCategory::Volume,
        source: "PVI:=IF(VOLUME>REF(VOLUME,1),REF(PVI,1)+(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*REF(PVI,1),REF(PVI,1)); MA_PVI:=MA(PVI,M); CROSS(PVI,MA_PVI)".to_string(),
        parameters: vec![("M".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("tsi_true".to_string(), FormulaTemplate {
        name: "TSI真实强度".to_string(),
        description: "真实强度指数，双重平滑动量".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MOM:=CLOSE-REF(CLOSE,1); SMOOTH1:=EMA(EMA(MOM,N),N); SMOOTH2:=EMA(EMA(ABS(MOM),N),N); TSI:=SMOOTH1/SMOOTH2*100; SIGNAL:=EMA(TSI,M); CROSS(TSI,SIGNAL)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 25.0), ("M".to_string(), 5.0, 20.0, 13.0)],
    });

    map.insert("uo_ultimate".to_string(), FormulaTemplate {
        name: "UO终极震荡".to_string(),
        description: "终极震荡指标，多周期加权".to_string(),
        category: TemplateCategory::Oscillator,
        source: "BP:=CLOSE-MIN(LOW,REF(CLOSE,1)); TR1:=MAX(HIGH,REF(CLOSE,1))-MIN(LOW,REF(CLOSE,1)); AVG7:=SUM(BP,7)/SUM(TR1,7); AVG14:=SUM(BP,14)/SUM(TR1,14); AVG28:=SUM(BP,28)/SUM(TR1,28); UO:=100*((4*AVG7+2*AVG14+AVG28)/7); UO<30".to_string(),
        parameters: vec![],
    });

    map.insert("ao_awesome".to_string(), FormulaTemplate {
        name: "AO动量震荡".to_string(),
        description: "动量震荡指标，比尔威廉姆斯".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MEDIAN:=(HIGH+LOW)/2; AO:=MA(MEDIAN,5)-MA(MEDIAN,34); CROSS(AO,0)".to_string(),
        parameters: vec![],
    });

    map.insert("ac_accelerator".to_string(), FormulaTemplate {
        name: "AC加速震荡".to_string(),
        description: "加速震荡指标，AO的动量".to_string(),
        category: TemplateCategory::Oscillator,
        source: "MEDIAN:=(HIGH+LOW)/2; AO:=MA(MEDIAN,5)-MA(MEDIAN,34); AC:=AO-MA(AO,5); CROSS(AC,0)".to_string(),
        parameters: vec![],
    });

    map.insert("gator_oscillator".to_string(), FormulaTemplate {
        name: "Gator鳄鱼线".to_string(),
        description: "鳄鱼线震荡指标，趋势休眠与活跃".to_string(),
        category: TemplateCategory::Trend,
        source: "JAW:=MA((HIGH+LOW)/2,13); TEETH:=MA((HIGH+LOW)/2,8); LIPS:=MA((HIGH+LOW)/2,5); GATOR_UP:=JAW-TEETH; GATOR_DOWN:=TEETH-LIPS; GATOR_UP>0 AND GATOR_DOWN>0".to_string(),
        parameters: vec![],
    });

    map.insert("fractal_break".to_string(), FormulaTemplate {
        name: "分形突破".to_string(),
        description: "比尔威廉姆斯分形突破".to_string(),
        category: TemplateCategory::Pattern,
        source: "UP_FRACTAL:=HIGH>REF(HIGH,1) AND HIGH>REF(HIGH,2) AND HIGH>REF(HIGH,-1) AND HIGH>REF(HIGH,-2); DOWN_FRACTAL:=LOW<REF(LOW,1) AND LOW<REF(LOW,2) AND LOW<REF(LOW,-1) AND LOW<REF(LOW,-2); CLOSE>REF(HHV(HIGH,5),1)".to_string(),
        parameters: vec![],
    });

    map.insert("alligator_trend".to_string(), FormulaTemplate {
        name: "鳄鱼线趋势".to_string(),
        description: "鳄鱼线趋势判断".to_string(),
        category: TemplateCategory::Trend,
        source: "JAW:=MA((HIGH+LOW)/2,13); TEETH:=MA((HIGH+LOW)/2,8); LIPS:=MA((HIGH+LOW)/2,5); CLOSE>JAW AND CLOSE>TEETH AND CLOSE>LIPS".to_string(),
        parameters: vec![],
    });

    map.insert("heikinashi_trend".to_string(), FormulaTemplate {
        name: "平均K线趋势".to_string(),
        description: "Heikin-Ashi平均K线趋势".to_string(),
        category: TemplateCategory::Trend,
        source: "HA_CLOSE:=(OPEN+HIGH+LOW+CLOSE)/4; HA_OPEN:=REF((HA_OPEN+HA_CLOSE)/2,1); HA_OPEN<HA_CLOSE".to_string(),
        parameters: vec![],
    });

    map.insert("ichimoku_cloud".to_string(), FormulaTemplate {
        name: "云图穿越".to_string(),
        description: "一目均衡表云图穿越".to_string(),
        category: TemplateCategory::Trend,
        source: "TENKAN:=(HHV(HIGH,9)+LLV(LOW,9))/2; KIJUN:=(HHV(HIGH,26)+LLV(LOW,26))/2; SENKOU_A:=(TENKAN+KIJUN)/2; SENKOU_B:=(HHV(HIGH,52)+LLV(LOW,52))/2; CLOSE>SENKOU_A AND CLOSE>SENKOU_B".to_string(),
        parameters: vec![],
    });

    map.insert("three_white_soldiers".to_string(), FormulaTemplate {
        name: "三白兵".to_string(),
        description: "三白兵看涨形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=CLOSE>OPEN AND (CLOSE-OPEN)/OPEN>0.02; C2:=REF(CLOSE,1)>REF(OPEN,1) AND (REF(CLOSE,1)-REF(OPEN,1))/REF(OPEN,1)>0.02; C3:=REF(CLOSE,2)>REF(OPEN,2) AND (REF(CLOSE,2)-REF(OPEN,2))/REF(OPEN,2)>0.02; UP_TREND:=CLOSE>REF(CLOSE,1) AND REF(CLOSE,1)>REF(CLOSE,2); C1 AND C2 AND C3 AND UP_TREND".to_string(),
        parameters: vec![],
    });

    map.insert("three_black_crows".to_string(), FormulaTemplate {
        name: "三乌鸦".to_string(),
        description: "三乌鸦看跌形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=CLOSE<OPEN AND (OPEN-CLOSE)/OPEN>0.02; C2:=REF(CLOSE,1)<REF(OPEN,1) AND (REF(OPEN,1)-REF(CLOSE,1))/REF(OPEN,1)>0.02; C3:=REF(CLOSE,2)<REF(OPEN,2) AND (REF(OPEN,2)-REF(CLOSE,2))/REF(OPEN,2)>0.02; DOWN_TREND:=CLOSE<REF(CLOSE,1) AND REF(CLOSE,1)<REF(CLOSE,2); C1 AND C2 AND C3 AND DOWN_TREND".to_string(),
        parameters: vec![],
    });

    map.insert("morning_star".to_string(), FormulaTemplate {
        name: "启明星".to_string(),
        description: "启明星看涨反转形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,2)<REF(OPEN,2) AND (REF(OPEN,2)-REF(CLOSE,2))/REF(OPEN,2)>0.02; C2:=REF(OPEN,1)<REF(CLOSE,2) AND ABS(REF(CLOSE,1)-REF(OPEN,1))<REF(OPEN,2)*0.01; C3:=CLOSE>OPEN AND CLOSE>(REF(OPEN,2)+REF(CLOSE,2))/2; C1 AND C2 AND C3".to_string(),
        parameters: vec![],
    });

    map.insert("evening_star".to_string(), FormulaTemplate {
        name: "黄昏星".to_string(),
        description: "黄昏星看跌反转形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,2)>REF(OPEN,2) AND (REF(CLOSE,2)-REF(OPEN,2))/REF(OPEN,2)>0.02; C2:=REF(OPEN,1)>REF(CLOSE,2) AND ABS(REF(CLOSE,1)-REF(OPEN,1))<REF(OPEN,2)*0.01; C3:=CLOSE<OPEN AND CLOSE<(REF(OPEN,2)+REF(CLOSE,2))/2; C1 AND C2 AND C3".to_string(),
        parameters: vec![],
    });

    map.insert("hammer_pattern".to_string(), FormulaTemplate {
        name: "锤子线".to_string(),
        description: "锤子线底部反转形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "BODY:=ABS(CLOSE-OPEN); LOWER_SHADOW:=MIN(CLOSE,OPEN)-LOW; UPPER_SHADOW:=HIGH-MAX(CLOSE,OPEN); LOWER_SHADOW>BODY*2 AND UPPER_SHADOW<BODY*0.5 AND BODY>0".to_string(),
        parameters: vec![],
    });

    map.insert("shooting_star".to_string(), FormulaTemplate {
        name: "流星线".to_string(),
        description: "流星线顶部反转形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "BODY:=ABS(CLOSE-OPEN); UPPER_SHADOW:=HIGH-MAX(CLOSE,OPEN); LOWER_SHADOW:=MIN(CLOSE,OPEN)-LOW; UPPER_SHADOW>BODY*2 AND LOWER_SHADOW<BODY*0.5 AND BODY>0".to_string(),
        parameters: vec![],
    });

    map.insert("engulfing_bull".to_string(), FormulaTemplate {
        name: "看涨吞没".to_string(),
        description: "看涨吞没形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,1)<REF(OPEN,1); C2:=CLOSE>OPEN; C3:=OPEN<REF(CLOSE,1) AND CLOSE>REF(OPEN,1); C1 AND C2 AND C3".to_string(),
        parameters: vec![],
    });

    map.insert("engulfing_bear".to_string(), FormulaTemplate {
        name: "看跌吞没".to_string(),
        description: "看跌吞没形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,1)>REF(OPEN,1); C2:=CLOSE<OPEN; C3:=OPEN>REF(CLOSE,1) AND CLOSE<REF(OPEN,1); C1 AND C2 AND C3".to_string(),
        parameters: vec![],
    });

    map.insert("piercing_line".to_string(), FormulaTemplate {
        name: "刺透形态".to_string(),
        description: "刺透形态看涨信号".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,1)<REF(OPEN,1); C2:=CLOSE>OPEN; C3:=OPEN<REF(LOW,1); C4:=CLOSE>REF(OPEN,1)+(REF(CLOSE,1)-REF(OPEN,1))/2; C1 AND C2 AND C3 AND C4".to_string(),
        parameters: vec![],
    });

    map.insert("dark_cloud".to_string(), FormulaTemplate {
        name: "乌云盖顶".to_string(),
        description: "乌云盖顶看跌信号".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,1)>REF(OPEN,1); C2:=CLOSE<OPEN; C3:=OPEN>REF(HIGH,1); C4:=CLOSE<REF(OPEN,1)+(REF(CLOSE,1)-REF(OPEN,1))/2; C1 AND C2 AND C3 AND C4".to_string(),
        parameters: vec![],
    });

    map.insert("doji_star".to_string(), FormulaTemplate {
        name: "十字星".to_string(),
        description: "十字星形态，市场犹豫".to_string(),
        category: TemplateCategory::Pattern,
        source: "BODY:=ABS(CLOSE-OPEN); RANGE:=HIGH-LOW; BODY<RANGE*0.1 AND RANGE>REF(RANGE,1)*0.5".to_string(),
        parameters: vec![],
    });

    map.insert("harami_bull".to_string(), FormulaTemplate {
        name: "看涨孕育".to_string(),
        description: "看涨孕育形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,1)<REF(OPEN,1); C2:=CLOSE>OPEN; C3:=OPEN>REF(CLOSE,1) AND CLOSE<REF(OPEN,1); C1 AND C2 AND C3".to_string(),
        parameters: vec![],
    });

    map.insert("harami_bear".to_string(), FormulaTemplate {
        name: "看跌孕育".to_string(),
        description: "看跌孕育形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,1)>REF(OPEN,1); C2:=CLOSE<OPEN; C3:=OPEN<REF(CLOSE,1) AND CLOSE>REF(OPEN,1); C1 AND C2 AND C3".to_string(),
        parameters: vec![],
    });

    map.insert("rising_three".to_string(), FormulaTemplate {
        name: "上升三法".to_string(),
        description: "上升三法持续形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,4)>REF(OPEN,4) AND (REF(CLOSE,4)-REF(OPEN,4))/REF(OPEN,4)>0.02; C2:=REF(CLOSE,3)<REF(OPEN,3) AND REF(CLOSE,3)>REF(OPEN,4); C3:=REF(CLOSE,2)<REF(OPEN,2) AND REF(CLOSE,2)>REF(OPEN,4); C4:=REF(CLOSE,1)<REF(OPEN,1) AND REF(CLOSE,1)>REF(OPEN,4); C5:=CLOSE>OPEN AND CLOSE>REF(CLOSE,4); C1 AND C2 AND C3 AND C4 AND C5".to_string(),
        parameters: vec![],
    });

    map.insert("falling_three".to_string(), FormulaTemplate {
        name: "下降三法".to_string(),
        description: "下降三法持续形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "C1:=REF(CLOSE,4)<REF(OPEN,4) AND (REF(OPEN,4)-REF(CLOSE,4))/REF(OPEN,4)>0.02; C2:=REF(CLOSE,3)>REF(OPEN,3) AND REF(CLOSE,3)<REF(OPEN,4); C3:=REF(CLOSE,2)>REF(OPEN,2) AND REF(CLOSE,2)<REF(OPEN,4); C4:=REF(CLOSE,1)>REF(OPEN,1) AND REF(CLOSE,1)<REF(OPEN,4); C5:=CLOSE<OPEN AND CLOSE<REF(CLOSE,4); C1 AND C2 AND C3 AND C4 AND C5".to_string(),
        parameters: vec![],
    });

    map.insert("turtle_breakout".to_string(), FormulaTemplate {
        name: "海龟交易突破".to_string(),
        description: "海龟交易法则突破策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "HH:=HHV(HIGH,N); LL:=LLV(LOW,N); BREAK_UP:=CLOSE>REF(HH,1); BREAK_DOWN:=CLOSE<REF(LL,1); BREAK_UP".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("turtle_exit".to_string(), FormulaTemplate {
        name: "海龟交易离场".to_string(),
        description: "海龟交易法则离场策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "LL:=LLV(LOW,N); EXIT:=CLOSE<REF(LL,1); EXIT".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 10.0)],
    });

    map.insert("dual_thrust".to_string(), FormulaTemplate {
        name: "Dual Thrust策略".to_string(),
        description: "Dual Thrust区间突破策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "HH:=HHV(HIGH,N); LL:=LLV(LOW,N); HC:=HHV(CLOSE,N); LC:=LLV(CLOSE,N); RANGE:=MAX(HH-LC,HC-LL); UPPER:=OPEN+K1*RANGE; LOWER:=OPEN-K2*RANGE; CLOSE>UPPER".to_string(),
        parameters: vec![("N".to_string(), 3.0, 10.0, 4.0), ("K1".to_string(), 0.3, 0.7, 0.4), ("K2".to_string(), 0.3, 0.7, 0.4)],
    });

    map.insert("r_breaker".to_string(), FormulaTemplate {
        name: "R-Breaker策略".to_string(),
        description: "R-Breaker反转突破策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "HH:=REF(HHV(HIGH,N),1); LL:=REF(LLV(LOW,N),1); CC:=REF(CLOSE,1); OBSERVE:=HH-LL; B_BREAK:=HH+OBSERVE*0.1; S_SETUP:=HH-OBSERVE*0.2; B_SETUP:=LL+OBSERVE*0.2; S_BREAK:=LL-OBSERVE*0.1; CLOSE>B_BREAK".to_string(),
        parameters: vec![("N".to_string(), 3.0, 10.0, 4.0)],
    });

    map.insert("grid_trading".to_string(), FormulaTemplate {
        name: "网格交易信号".to_string(),
        description: "网格交易策略信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "BASE:=MA(CLOSE,N); GRID_PCT:=PCT/100; UPPER1:=BASE*(1+GRID_PCT); LOWER1:=BASE*(1-GRID_PCT); CLOSE<LOWER1".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0), ("PCT".to_string(), 1.0, 5.0, 2.0)],
    });

    map.insert("momentum_breakout".to_string(), FormulaTemplate {
        name: "动量突破策略".to_string(),
        description: "动量突破交易策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MOM:=CLOSE-REF(CLOSE,N); MOM_MA:=MA(MOM,M); HH:=HHV(HIGH,N); MOM>0 AND MOM>MOM_MA AND CLOSE>REF(HH,1)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0), ("M".to_string(), 3.0, 10.0, 5.0)],
    });

    map.insert("reversal_catch".to_string(), FormulaTemplate {
        name: "反转捕捉策略".to_string(),
        description: "捕捉价格反转机会".to_string(),
        category: TemplateCategory::Strategy,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; RSI<30 AND CLOSE>REF(CLOSE,1) AND VOLUME>MA(VOLUME,5)".to_string(),
        parameters: vec![],
    });

    map.insert("volatility_breakout".to_string(), FormulaTemplate {
        name: "波动率突破".to_string(),
        description: "基于波动率的突破策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "TR:=MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))); ATR:=MA(TR,N); RANGE:=ATR*MULT; UPPER:=REF(CLOSE,1)+RANGE; LOWER:=REF(CLOSE,1)-RANGE; CLOSE>UPPER".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 14.0), ("MULT".to_string(), 1.0, 3.0, 2.0)],
    });

    map.insert("pair_trading_signal".to_string(), FormulaTemplate {
        name: "配对交易信号".to_string(),
        description: "配对交易均值回归信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "SPREAD:=CLOSE/REF(CLOSE,1)-MA(CLOSE/REF(CLOSE,1),N); STD_SPREAD:=STD(SPREAD,N); Z_SCORE:=SPREAD/STD_SPREAD; Z_SCORE<-2".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("scalping_signal".to_string(), FormulaTemplate {
        name: "日内短线信号".to_string(),
        description: "日内短线交易信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA_FAST:=MA(CLOSE,5); MA_SLOW:=MA(CLOSE,20); VOL_UP:=VOLUME>MA(VOLUME,5)*1.5; TREND:=MA_FAST>MA_SLOW; CROSS(MA_FAST,MA_SLOW) AND VOL_UP".to_string(),
        parameters: vec![],
    });

    map.insert("swing_trade_signal".to_string(), FormulaTemplate {
        name: "波段交易信号".to_string(),
        description: "波段交易入场信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MA20>MA60 AND RSI<40 AND CLOSE>MA20".to_string(),
        parameters: vec![],
    });

    map.insert("position_management".to_string(), FormulaTemplate {
        name: "仓位管理信号".to_string(),
        description: "基于趋势强度的仓位管理".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); STRONG:=MA5>MA20 AND MA20>MA60; TREND_UP:=MA5>REF(MA5,1); STRONG AND TREND_UP".to_string(),
        parameters: vec![],
    });

    map.insert("risk_control_signal".to_string(), FormulaTemplate {
        name: "风控信号".to_string(),
        description: "风险控制预警信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),14); DRAWDOWN:=(HHV(CLOSE,20)-CLOSE)/HHV(CLOSE,20)*100; DRAWDOWN>ATR/MA20*100*3".to_string(),
        parameters: vec![],
    });

    map.insert("profit_taking_signal".to_string(), FormulaTemplate {
        name: "止盈信号".to_string(),
        description: "动态止盈信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); PROFIT_PCT:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; PROFIT_PCT>15 AND CLOSE<MA20".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 10.0)],
    });

    map.insert("stop_loss_signal".to_string(), FormulaTemplate {
        name: "止损信号".to_string(),
        description: "动态止损信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); LOSS_PCT:=(REF(CLOSE,N)-CLOSE)/REF(CLOSE,N)*100; LOSS_PCT>8 AND CLOSE<MA20".to_string(),
        parameters: vec![("N".to_string(), 3.0, 15.0, 5.0)],
    });

    map.insert("trailing_stop".to_string(), FormulaTemplate {
        name: "移动止损".to_string(),
        description: "追踪止损策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "HH:=HHV(HIGH,N); TRAIL_STOP:=HH*(1-PCT/100); CLOSE<TRAIL_STOP".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 10.0), ("PCT".to_string(), 3.0, 15.0, 8.0)],
    });

    map.insert("time_exit".to_string(), FormulaTemplate {
        name: "时间离场".to_string(),
        description: "基于时间的离场策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "HOLD_DAYS:=BARSLAST(CROSS(MA(CLOSE,5),MA(CLOSE,10))); HOLD_DAYS>N AND CLOSE<MA(CLOSE,5)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 10.0)],
    });

    map.insert("volume_profile".to_string(), FormulaTemplate {
        name: "成交量分布".to_string(),
        description: "成交量分布分析".to_string(),
        category: TemplateCategory::Volume,
        source: "PRICE_LEVEL:=CLOSE; VOL_AT_LEVEL:=SUM(IF(ABS(CLOSE-PRICE_LEVEL)<PRICE_LEVEL*0.01,VOLUME,0),N); TOTAL_VOL:=SUM(VOLUME,N); VOL_PCT:=VOL_AT_LEVEL/TOTAL_VOL*100; VOL_PCT>5".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("vwap_strategy".to_string(), FormulaTemplate {
        name: "VWAP交易策略".to_string(),
        description: "成交量加权均价交易策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "VWAP:=SUM(AMOUNT,N)/SUM(VOLUME,N); CROSS(CLOSE,VWAP) AND VOLUME>MA(VOLUME,5)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 20.0)],
    });

    map.insert("twap_strategy".to_string(), FormulaTemplate {
        name: "TWAP交易策略".to_string(),
        description: "时间加权均价交易策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "TWAP:=MA(CLOSE,N); CROSS(CLOSE,TWAP) AND VOLUME>MA(VOLUME,5)".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 20.0)],
    });

    map.insert("pivot_point".to_string(), FormulaTemplate {
        name: "枢轴点策略".to_string(),
        description: "枢轴点支撑阻力策略".to_string(),
        category: TemplateCategory::Classic,
        source: "PP:=(REF(HIGH,1)+REF(LOW,1)+REF(CLOSE,1))/3; R1:=2*PP-REF(LOW,1); S1:=2*PP-REF(HIGH,1); R2:=PP+(REF(HIGH,1)-REF(LOW,1)); S2:=PP-(REF(HIGH,1)-REF(LOW,1)); CROSS(CLOSE,R1)".to_string(),
        parameters: vec![],
    });

    map.insert("fibonacci_retracement".to_string(), FormulaTemplate {
        name: "斐波那契回撤".to_string(),
        description: "斐波那契回撤位策略".to_string(),
        category: TemplateCategory::Classic,
        source: "HH:=HHV(HIGH,N); LL:=LLV(LOW,N); RANGE:=HH-LL; RETRACE_382:=HH-RANGE*0.382; RETRACE_618:=HH-RANGE*0.618; CLOSE>RETRACE_382 AND CLOSE<RETRACE_618".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("elliott_wave".to_string(), FormulaTemplate {
        name: "波浪计数".to_string(),
        description: "艾略特波浪理论辅助".to_string(),
        category: TemplateCategory::Classic,
        source: "MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); WAVE_UP:=MA5>REF(MA5,1) AND MA20>REF(MA20,1); WAVE_DOWN:=MA5<REF(MA5,1) AND MA20<REF(MA20,1); WAVE_UP AND REF(WAVE_DOWN,5)".to_string(),
        parameters: vec![],
    });

    map.insert("gann_angle".to_string(), FormulaTemplate {
        name: "江恩角度线".to_string(),
        description: "江恩角度线支撑阻力".to_string(),
        category: TemplateCategory::Classic,
        source: "LOW_N:=LLV(LOW,N); GANN_1X1:=LOW_N+(HIGH-LOW_N)*0.5; GANN_2X1:=LOW_N+(HIGH-LOW_N)*0.333; CLOSE>GANN_1X1".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("harmonic_pattern".to_string(), FormulaTemplate {
        name: "谐波形态".to_string(),
        description: "谐波形态识别".to_string(),
        category: TemplateCategory::Pattern,
        source: "X:=REF(LOW,4); A:=REF(HIGH,3); B:=REF(LOW,2); C:=REF(HIGH,1); AB_RATIO:=(A-B)/(A-X); BC_RATIO:=(C-B)/(A-B); AB_RATIO>0.382 AND AB_RATIO<0.886 AND BC_RATIO>0.382 AND BC_RATIO<0.886".to_string(),
        parameters: vec![],
    });

    map.insert("wyckoff_accumulation".to_string(), FormulaTemplate {
        name: "威科夫吸筹".to_string(),
        description: "威科夫吸筹区间识别".to_string(),
        category: TemplateCategory::Pattern,
        source: "PS:=VOLUME>MA(VOLUME,20)*2 AND CLOSE>REF(CLOSE,1); SC:=VOLUME>MA(VOLUME,20)*1.5 AND CLOSE<REF(CLOSE,1); AR:=VOLUME>MA(VOLUME,20) AND CLOSE>REF(CLOSE,1); LOW_RANGE:=HHV(HIGH,20)-LLV(LOW,20); LOW_RANGE<MA(LOW_RANGE,60)*0.5".to_string(),
        parameters: vec![],
    });

    map.insert("wyckoff_distribution".to_string(), FormulaTemplate {
        name: "威科夫派发".to_string(),
        description: "威科夫派发区间识别".to_string(),
        category: TemplateCategory::Pattern,
        source: "PSY:=VOLUME>MA(VOLUME,20)*2 AND CLOSE<REF(CLOSE,1); BC:=VOLUME>MA(VOLUME,20)*1.5 AND CLOSE>REF(CLOSE,1); AR:=VOLUME>MA(VOLUME,20) AND CLOSE<REF(CLOSE,1); HIGH_RANGE:=HHV(HIGH,20)-LLV(LOW,20); HIGH_RANGE<MA(HIGH_RANGE,60)*0.5".to_string(),
        parameters: vec![],
    });

    map.insert("market_profile".to_string(), FormulaTemplate {
        name: "市场轮廓".to_string(),
        description: "市场轮廓价值区域".to_string(),
        category: TemplateCategory::Classic,
        source: "POC:=MA(CLOSE,N); VALUE_HIGH:=POC+STD(CLOSE,N); VALUE_LOW:=POC-STD(CLOSE,N); CLOSE>VALUE_HIGH OR CLOSE<VALUE_LOW".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("volume_spread".to_string(), FormulaTemplate {
        name: "量价差分析".to_string(),
        description: "成交量与价差关系分析".to_string(),
        category: TemplateCategory::Volume,
        source: "SPREAD:=HIGH-LOW; VOL_MA:=MA(VOLUME,N); SPREAD_MA:=MA(SPREAD,N); VOL_UP:=VOLUME>VOL_MA*1.5; SPREAD_DOWN:=SPREAD<SPREAD_MA*0.7; VOL_UP AND SPREAD_DOWN".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0)],
    });

    map.insert("effort_result".to_string(), FormulaTemplate {
        name: "努力结果分析".to_string(),
        description: "威科夫努力与结果分析".to_string(),
        category: TemplateCategory::Volume,
        source: "EFFORT:=VOLUME/MA(VOLUME,10); RESULT:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100; EFFORT_UP:=EFFORT>1.5; RESULT_DOWN:=RESULT<0.5; EFFORT_UP AND RESULT_DOWN".to_string(),
        parameters: vec![],
    });

    map.insert("spring_pattern".to_string(), FormulaTemplate {
        name: "弹簧形态".to_string(),
        description: "威科夫弹簧形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "LL:=LLV(LOW,20); SPRING:=LOW<LL AND CLOSE>LL; VOL_LOW:=VOLUME<MA(VOLUME,10); SPRING AND VOL_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("upthrust_pattern".to_string(), FormulaTemplate {
        name: "上冲回落".to_string(),
        description: "威科夫上冲回落形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "HH:=HHV(HIGH,20); UPTHRUST:=HIGH>HH AND CLOSE<HH; VOL_HIGH:=VOLUME>MA(VOLUME,10); UPTHRUST AND VOL_HIGH".to_string(),
        parameters: vec![],
    });

    map.insert("no_demand".to_string(), FormulaTemplate {
        name: "无需求".to_string(),
        description: "威科夫无需求形态".to_string(),
        category: TemplateCategory::Volume,
        source: "PRICE_DOWN:=CLOSE<REF(CLOSE,1); VOL_LOW:=VOLUME<MA(VOLUME,10)*0.5; SPREAD_LOW:=(HIGH-LOW)<MA(HIGH-LOW,10)*0.7; PRICE_DOWN AND VOL_LOW AND SPREAD_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("no_supply".to_string(), FormulaTemplate {
        name: "无供给".to_string(),
        description: "威科夫无供给形态".to_string(),
        category: TemplateCategory::Volume,
        source: "PRICE_UP:=CLOSE>REF(CLOSE,1); VOL_LOW:=VOLUME<MA(VOLUME,10)*0.5; SPREAD_LOW:=(HIGH-LOW)<MA(HIGH-LOW,10)*0.7; PRICE_UP AND VOL_LOW AND SPREAD_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("stopping_volume".to_string(), FormulaTemplate {
        name: "止损量".to_string(),
        description: "威科夫止损量形态".to_string(),
        category: TemplateCategory::Volume,
        source: "PRICE_DOWN:=CLOSE<REF(CLOSE,1); VOL_HIGH:=VOLUME>MA(VOLUME,10)*2; CLOSE_NEAR_LOW:=CLOSE>LOW+(HIGH-LOW)*0.5; PRICE_DOWN AND VOL_HIGH AND CLOSE_NEAR_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("climax_volume".to_string(), FormulaTemplate {
        name: "高潮量".to_string(),
        description: "成交量高潮识别".to_string(),
        category: TemplateCategory::Volume,
        source: "VOL_EXTREME:=VOLUME>MA(VOLUME,20)*3; PRICE_EXTREME:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100)>5; VOL_EXTREME AND PRICE_EXTREME".to_string(),
        parameters: vec![],
    });

    map.insert("churn_bar".to_string(), FormulaTemplate {
        name: "搅动K线".to_string(),
        description: "高量窄幅搅动K线".to_string(),
        category: TemplateCategory::Volume,
        source: "VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; SPREAD_LOW:=(HIGH-LOW)<MA(HIGH-LOW,10)*0.5; VOL_HIGH AND SPREAD_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("test_bar".to_string(), FormulaTemplate {
        name: "测试K线".to_string(),
        description: "威科夫测试K线形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "LOW_TEST:=LOW<REF(LOW,1); VOL_LOW:=VOLUME<MA(VOLUME,10)*0.7; CLOSE_UP:=CLOSE>REF(CLOSE,1); LOW_TEST AND VOL_LOW AND CLOSE_UP".to_string(),
        parameters: vec![],
    });

    map.insert("secondary_test".to_string(), FormulaTemplate {
        name: "二次测试".to_string(),
        description: "威科夫二次测试形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "FIRST_LOW:=REF(LOW,5)=LLV(LOW,10); SECOND_LOW:=LOW<REF(LOW,1)*1.01; VOL_DECREASE:=VOLUME<REF(VOLUME,5); FIRST_LOW AND SECOND_LOW AND VOL_DECREASE".to_string(),
        parameters: vec![],
    });

    map.insert("sos_sign".to_string(), FormulaTemplate {
        name: "SOS信号".to_string(),
        description: "威科夫强势信号".to_string(),
        category: TemplateCategory::Pattern,
        source: "PRICE_UP:=CLOSE>REF(CLOSE,1)*1.02; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; SPREAD_HIGH:=(HIGH-LOW)>MA(HIGH-LOW,10)*1.3; PRICE_UP AND VOL_HIGH AND SPREAD_HIGH".to_string(),
        parameters: vec![],
    });

    map.insert("lps_pattern".to_string(), FormulaTemplate {
        name: "LPS形态".to_string(),
        description: "威科夫最后支撑点".to_string(),
        category: TemplateCategory::Pattern,
        source: "HH:=HHV(HIGH,20); NEAR_HIGH:=CLOSE>HH*0.95; VOL_LOW:=VOLUME<MA(VOLUME,10)*0.7; PULLBACK:=CLOSE<REF(CLOSE,1); NEAR_HIGH AND VOL_LOW AND PULLBACK".to_string(),
        parameters: vec![],
    });

    map.insert("sow_pattern".to_string(), FormulaTemplate {
        name: "SOW形态".to_string(),
        description: "威科夫弱势信号".to_string(),
        category: TemplateCategory::Pattern,
        source: "PRICE_DOWN:=CLOSE<REF(CLOSE,1)*0.98; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; SPREAD_HIGH:=(HIGH-LOW)>MA(HIGH-LOW,10)*1.3; PRICE_DOWN AND VOL_HIGH AND SPREAD_HIGH".to_string(),
        parameters: vec![],
    });

    map.insert("ps_pattern".to_string(), FormulaTemplate {
        name: "PS形态".to_string(),
        description: "威科夫初步支撑".to_string(),
        category: TemplateCategory::Pattern,
        source: "DOWNTREND:=MA(CLOSE,10)<MA(CLOSE,20); VOL_HIGH:=VOLUME>MA(VOLUME,20)*2; PRICE_BOUNCE:=CLOSE>REF(CLOSE,1); DOWNTREND AND VOL_HIGH AND PRICE_BOUNCE".to_string(),
        parameters: vec![],
    });

    map.insert("sc_pattern".to_string(), FormulaTemplate {
        name: "SC形态".to_string(),
        description: "威科夫卖出高潮".to_string(),
        category: TemplateCategory::Pattern,
        source: "VOL_EXTREME:=VOLUME>MA(VOLUME,20)*3; PRICE_PANIC:=CLOSE<REF(CLOSE,1)*0.95; RECOVERY:=CLOSE>LOW+(HIGH-LOW)*0.5; VOL_EXTREME AND PRICE_PANIC AND RECOVERY".to_string(),
        parameters: vec![],
    });

    map.insert("ar_pattern".to_string(), FormulaTemplate {
        name: "AR形态".to_string(),
        description: "威科夫自动反弹".to_string(),
        category: TemplateCategory::Pattern,
        source: "AFTER_SC:=REF(LOW,1)=LLV(LOW,10); PRICE_UP:=CLOSE>REF(CLOSE,1)*1.02; VOL_MODERATE:=VOLUME>MA(VOLUME,10)*0.8 AND VOLUME<MA(VOLUME,10)*1.5; AFTER_SC AND PRICE_UP AND VOL_MODERATE".to_string(),
        parameters: vec![],
    });

    map.insert("st_pattern".to_string(), FormulaTemplate {
        name: "ST形态".to_string(),
        description: "威科夫二次测试".to_string(),
        category: TemplateCategory::Pattern,
        source: "AR_HIGH:=REF(HIGH,3)=HHV(HIGH,10); RETEST_LOW:=LOW<=REF(LOW,3)*1.01; VOL_LOWER:=VOLUME<REF(VOLUME,3); AR_HIGH AND RETEST_LOW AND VOL_LOWER".to_string(),
        parameters: vec![],
    });

    map.insert("utad_pattern".to_string(), FormulaTemplate {
        name: "UTAD形态".to_string(),
        description: "威科夫终极洗盘".to_string(),
        category: TemplateCategory::Pattern,
        source: "RANGE_HIGH:=HIGH>HHV(HIGH,20); FAIL_HOLD:=CLOSE<REF(HIGH,20); VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; RANGE_HIGH AND FAIL_HOLD AND VOL_HIGH".to_string(),
        parameters: vec![],
    });

    map.insert("ice_breaking".to_string(), FormulaTemplate {
        name: "冰线突破".to_string(),
        description: "威科夫冰线突破形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "ICE_LEVEL:=MA(LOW,20); BREAK_DOWN:=CLOSE<ICE_LEVEL; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; NO_RECOVERY:=CLOSE<OPEN; BREAK_DOWN AND VOL_HIGH AND NO_RECOVERY".to_string(),
        parameters: vec![],
    });

    map.insert("jump_across_creek".to_string(), FormulaTemplate {
        name: "跳过小溪".to_string(),
        description: "威科夫跳过小溪形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "RESISTANCE:=HHV(HIGH,20); BREAK_UP:=CLOSE>RESISTANCE; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; CONFIRM:=CLOSE>OPEN; BREAK_UP AND VOL_HIGH AND CONFIRM".to_string(),
        parameters: vec![],
    });

    map.insert("backup_shallow".to_string(), FormulaTemplate {
        name: "浅回调".to_string(),
        description: "威科夫浅回调形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "BREAK_HIGH:=REF(CLOSE,1)>REF(HHV(HIGH,20),1); PULLBACK:=CLOSE<REF(CLOSE,1); SHALLOW:=CLOSE>REF(CLOSE,1)*0.97; VOL_LOW:=VOLUME<MA(VOLUME,10)*0.7; BREAK_HIGH AND PULLBACK AND SHALLOW AND VOL_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("backup_deep".to_string(), FormulaTemplate {
        name: "深回调".to_string(),
        description: "威科夫深回调形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "BREAK_HIGH:=REF(CLOSE,2)>REF(HHV(HIGH,20),2); DEEP_PULLBACK:=CLOSE<REF(CLOSE,2)*0.95; VOL_MODERATE:=VOLUME>MA(VOLUME,10)*0.5; BREAK_HIGH AND DEEP_PULLBACK AND VOL_MODERATE".to_string(),
        parameters: vec![],
    });

    map.insert("shakeout_pattern".to_string(), FormulaTemplate {
        name: "震仓形态".to_string(),
        description: "威科夫震仓形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "SHARP_DROP:=LOW<REF(LOW,1)*0.95; QUICK_RECOVERY:=CLOSE>REF(CLOSE,1); VOL_SPIKE:=VOLUME>MA(VOLUME,10)*2; SHARP_DROP AND QUICK_RECOVERY AND VOL_SPIKE".to_string(),
        parameters: vec![],
    });

    map.insert("accumulation_zone".to_string(), FormulaTemplate {
        name: "吸筹区间".to_string(),
        description: "威科夫吸筹区间识别".to_string(),
        category: TemplateCategory::Pattern,
        source: "RANGE:=HHV(HIGH,N)-LLV(LOW,N); RANGE_PCT:=RANGE/LLV(LOW,N)*100; NARROW_RANGE:=RANGE_PCT<15; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,20); NARROW_RANGE AND VOL_DECLINE".to_string(),
        parameters: vec![("N".to_string(), 10.0, 40.0, 20.0)],
    });

    map.insert("distribution_zone".to_string(), FormulaTemplate {
        name: "派发区间".to_string(),
        description: "威科夫派发区间识别".to_string(),
        category: TemplateCategory::Pattern,
        source: "RANGE:=HHV(HIGH,N)-LLV(LOW,N); RANGE_PCT:=RANGE/LLV(LOW,N)*100; NARROW_RANGE:=RANGE_PCT<15; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,20); HIGH_PRICE:=CLOSE>MA(CLOSE,60); NARROW_RANGE AND VOL_DECLINE AND HIGH_PRICE".to_string(),
        parameters: vec![("N".to_string(), 10.0, 40.0, 20.0)],
    });

    map.insert("markdown_phase".to_string(), FormulaTemplate {
        name: "下跌阶段".to_string(),
        description: "威科夫下跌阶段识别".to_string(),
        category: TemplateCategory::Trend,
        source: "MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); DOWN_TREND:=MA20<MA60 AND MA60<REF(MA60,5); VOL_INCREASE:=MA(VOLUME,5)>MA(VOLUME,20); DOWN_TREND AND VOL_INCREASE".to_string(),
        parameters: vec![],
    });

    map.insert("markup_phase".to_string(), FormulaTemplate {
        name: "上涨阶段".to_string(),
        description: "威科夫上涨阶段识别".to_string(),
        category: TemplateCategory::Trend,
        source: "MA20:=MA(CLOSE,20); MA60:=MA(CLOSE,60); UP_TREND:=MA20>MA60 AND MA60>REF(MA60,5); VOL_INCREASE:=MA(VOLUME,5)>MA(VOLUME,20); UP_TREND AND VOL_INCREASE".to_string(),
        parameters: vec![],
    });

    map.insert("reaccumulation".to_string(), FormulaTemplate {
        name: "再吸筹".to_string(),
        description: "威科夫再吸筹形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "UPTREND:=MA(CLOSE,10)>MA(CLOSE,30); CONSOLIDATION:=HHV(HIGH,10)-LLV(LOW,10)<MA(CLOSE,10)*0.05; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,10); UPTREND AND CONSOLIDATION AND VOL_DECLINE".to_string(),
        parameters: vec![],
    });

    map.insert("redistribution".to_string(), FormulaTemplate {
        name: "再派发".to_string(),
        description: "威科夫再派发形态".to_string(),
        category: TemplateCategory::Pattern,
        source: "DOWNTREND:=MA(CLOSE,10)<MA(CLOSE,30); CONSOLIDATION:=HHV(HIGH,10)-LLV(LOW,10)<MA(CLOSE,10)*0.05; VOL_DECLINE:=MA(VOLUME,5)<MA(VOLUME,10); DOWNTREND AND CONSOLIDATION AND VOL_DECLINE".to_string(),
        parameters: vec![],
    });

    map.insert("cause_effect".to_string(), FormulaTemplate {
        name: "因果分析".to_string(),
        description: "威科夫因果分析".to_string(),
        category: TemplateCategory::Classic,
        source: "TRADING_RANGE:=HHV(HIGH,N)-LLV(LOW,N); CAUSE:=TRADING_RANGE/LLV(LOW,N)*100; TARGET_MOVE:=CAUSE*1.5; CURRENT_MOVE:=ABS(CLOSE-LLV(LOW,N))/LLV(LOW,N)*100; CURRENT_MOVE<TARGET_MOVE".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("composite_operator".to_string(), FormulaTemplate {
        name: "综合操作者".to_string(),
        description: "威科夫综合操作者分析".to_string(),
        category: TemplateCategory::Classic,
        source: "SMART_MONEY_BUY:=VOLUME>MA(VOLUME,20)*2 AND CLOSE>REF(CLOSE,1); SMART_MONEY_SELL:=VOLUME>MA(VOLUME,20)*2 AND CLOSE<REF(CLOSE,1); NET_SMART:=SUM(IF(SMART_MONEY_BUY,1,IF(SMART_MONEY_SELL,-1,0)),10); NET_SMART>3".to_string(),
        parameters: vec![],
    });

    map.insert("law_of_supply_demand".to_string(), FormulaTemplate {
        name: "供需法则".to_string(),
        description: "威科夫供需法则分析".to_string(),
        category: TemplateCategory::Classic,
        source: "DEMAND:=IF(CLOSE>REF(CLOSE,1),VOLUME,0); SUPPLY:=IF(CLOSE<REF(CLOSE,1),VOLUME,0); DEMAND_MA:=MA(DEMAND,10); SUPPLY_MA:=MA(SUPPLY,10); DEMAND_MA>SUPPLY_MA*1.5".to_string(),
        parameters: vec![],
    });

    map.insert("law_of_cause_effect".to_string(), FormulaTemplate {
        name: "因果法则".to_string(),
        description: "威科夫因果法则分析".to_string(),
        category: TemplateCategory::Classic,
        source: "RANGE_DAYS:=HHV(HIGH,N)-LLV(LOW,N); CAUSE_DAYS:=N; EFFECT_MIN:=RANGE_DAYS*1.5; EFFECT_MAX:=RANGE_DAYS*3; CURRENT_MOVE:=ABS(CLOSE-REF(CLOSE,N)); CURRENT_MOVE>EFFECT_MIN AND CURRENT_MOVE<EFFECT_MAX".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("law_of_effort_result".to_string(), FormulaTemplate {
        name: "努力结果法则".to_string(),
        description: "威科夫努力结果法则分析".to_string(),
        category: TemplateCategory::Classic,
        source: "EFFORT:=VOLUME/MA(VOLUME,10); RESULT:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1)*100); EFFORT_RESULT_RATIO:=RESULT/EFFORT; EFFORT_RESULT_RATIO>0.5 AND EFFORT_RESULT_RATIO<2".to_string(),
        parameters: vec![],
    });

    map.insert("left_transaction".to_string(), FormulaTemplate {
        name: "左侧交易".to_string(),
        description: "左侧交易信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; BOTTOM_ZONE:=RSI<30; VOL_SHRINK:=VOLUME<MA(VOLUME,10)*0.7; BOTTOM_ZONE AND VOL_SHRINK".to_string(),
        parameters: vec![],
    });

    map.insert("right_transaction".to_string(), FormulaTemplate {
        name: "右侧交易".to_string(),
        description: "右侧交易信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); CONFIRM_UP:=CROSS(MA5,MA20); VOL_CONFIRM:=VOLUME>MA(VOLUME,5); CONFIRM_UP AND VOL_CONFIRM".to_string(),
        parameters: vec![],
    });

    map.insert("pyramid_buy".to_string(), FormulaTemplate {
        name: "金字塔买入".to_string(),
        description: "金字塔加仓策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); TREND_UP:=CLOSE>MA20 AND MA20>REF(MA20,5); PULLBACK:=CLOSE<REF(CLOSE,1); SUPPORT_HOLD:=CLOSE>MA20*0.98; TREND_UP AND PULLBACK AND SUPPORT_HOLD".to_string(),
        parameters: vec![],
    });

    map.insert("martingale_strategy".to_string(), FormulaTemplate {
        name: "马丁策略".to_string(),
        description: "马丁格尔策略信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "LOSS_PCT:=(REF(CLOSE,5)-CLOSE)/REF(CLOSE,5)*100; LOSS_PCT>10 AND VOLUME>MA(VOLUME,10)*1.5".to_string(),
        parameters: vec![],
    });

    map.insert("anti_martingale".to_string(), FormulaTemplate {
        name: "反马丁策略".to_string(),
        description: "反马丁格尔策略信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "PROFIT_PCT:=(CLOSE-REF(CLOSE,5))/REF(CLOSE,5)*100; PROFIT_PCT>5 AND VOLUME>MA(VOLUME,10)*1.2".to_string(),
        parameters: vec![],
    });

    map.insert("kelly_criterion".to_string(), FormulaTemplate {
        name: "凯利公式".to_string(),
        description: "凯利公式仓位计算".to_string(),
        category: TemplateCategory::Strategy,
        source: "WIN_RATE:=COUNT(CLOSE>REF(CLOSE,1),N)/N; AVG_WIN:=SUM(IF(CLOSE>REF(CLOSE,1),CLOSE/REF(CLOSE,1)-1,0),N)/COUNT(CLOSE>REF(CLOSE,1),N); AVG_LOSS:=SUM(IF(CLOSE<REF(CLOSE,1),REF(CLOSE,1)/CLOSE-1,0),N)/COUNT(CLOSE<REF(CLOSE,1),N); KELLY:=WIN_RATE-(1-WIN_RATE)/AVG_LOSS*AVG_WIN; KELLY>0.2".to_string(),
        parameters: vec![("N".to_string(), 20.0, 60.0, 30.0)],
    });

    map.insert("position_sizing".to_string(), FormulaTemplate {
        name: "仓位计算".to_string(),
        description: "基于波动率的仓位计算".to_string(),
        category: TemplateCategory::Strategy,
        source: "ATR:=MA(MAX(MAX(HIGH-LOW,ABS(HIGH-REF(CLOSE,1))),ABS(LOW-REF(CLOSE,1))),14); RISK_PCT:=2; POSITION_SIZE:=RISK_PCT/100/(ATR/CLOSE*100); POSITION_SIZE>0.02".to_string(),
        parameters: vec![],
    });

    map.insert("risk_reward_ratio".to_string(), FormulaTemplate {
        name: "风险收益比".to_string(),
        description: "风险收益比评估".to_string(),
        category: TemplateCategory::Strategy,
        source: "ENTRY:=CLOSE; STOP:=LLV(LOW,10); TARGET:=HHV(HIGH,20); RISK:=ENTRY-STOP; REWARD:=TARGET-ENTRY; RR_RATIO:=REWARD/RISK; RR_RATIO>2".to_string(),
        parameters: vec![],
    });

    map.insert("expectancy_calc".to_string(), FormulaTemplate {
        name: "期望值计算".to_string(),
        description: "交易期望值计算".to_string(),
        category: TemplateCategory::Strategy,
        source: "WIN_RATE:=COUNT(CLOSE>REF(CLOSE,1),N)/N; AVG_WIN:=SUM(IF(CLOSE>REF(CLOSE,1),CLOSE/REF(CLOSE,1)-1,0),N)/COUNT(CLOSE>REF(CLOSE,1),N); AVG_LOSS:=SUM(IF(CLOSE<REF(CLOSE,1),REF(CLOSE,1)/CLOSE-1,0),N)/COUNT(CLOSE<REF(CLOSE,1),N); EXPECTANCY:=WIN_RATE*AVG_WIN-(1-WIN_RATE)*AVG_LOSS; EXPECTANCY>0".to_string(),
        parameters: vec![("N".to_string(), 20.0, 60.0, 30.0)],
    });

    map.insert("sharpe_signal".to_string(), FormulaTemplate {
        name: "夏普比率信号".to_string(),
        description: "基于夏普比率的信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "RETURNS:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); AVG_RET:=MA(RETURNS,N); STD_RET:=STD(RETURNS,N); SHARPE:=AVG_RET/STD_RET; SHARPE>1".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("sortino_signal".to_string(), FormulaTemplate {
        name: "索提诺比率信号".to_string(),
        description: "基于索提诺比率的信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "RETURNS:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); AVG_RET:=MA(RETURNS,N); DOWN_RET:=IF(RETURNS<0,RETURNS,0); DOWN_STD:=STD(DOWN_RET,N); SORTINO:=AVG_RET/DOWN_STD; SORTINO>1.5".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("max_drawdown".to_string(), FormulaTemplate {
        name: "最大回撤".to_string(),
        description: "最大回撤监控".to_string(),
        category: TemplateCategory::Strategy,
        source: "PEAK:=HHV(CLOSE,N); DRAWDOWN:=(PEAK-CLOSE)/PEAK*100; MAX_DD:=HHV(DRAWDOWN,N); MAX_DD<20".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 30.0)],
    });

    map.insert("calmar_ratio".to_string(), FormulaTemplate {
        name: "卡玛比率".to_string(),
        description: "卡玛比率评估".to_string(),
        category: TemplateCategory::Strategy,
        source: "RETURNS:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); ANN_RET:=MA(RETURNS,N)*252; PEAK:=HHV(CLOSE,N); DRAWDOWN:=(PEAK-CLOSE)/PEAK; MAX_DD:=HHV(DRAWDOWN,N); CALMAR:=ANN_RET/MAX_DD; CALMAR>3".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 30.0)],
    });

    map.insert("win_streak".to_string(), FormulaTemplate {
        name: "连胜统计".to_string(),
        description: "连续盈利统计".to_string(),
        category: TemplateCategory::Strategy,
        source: "UP_DAYS:=COUNT(CLOSE>REF(CLOSE,1),N); STREAK:=UP_DAYS/N*100; STREAK>60".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0)],
    });

    map.insert("loss_streak".to_string(), FormulaTemplate {
        name: "连亏统计".to_string(),
        description: "连续亏损统计".to_string(),
        category: TemplateCategory::Strategy,
        source: "DOWN_DAYS:=COUNT(CLOSE<REF(CLOSE,1),N); STREAK:=DOWN_DAYS/N*100; STREAK>60".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0)],
    });

    map.insert("trade_frequency".to_string(), FormulaTemplate {
        name: "交易频率".to_string(),
        description: "交易频率控制".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); SIGNAL:=CROSS(MA5,MA20); SIGNAL_COUNT:=COUNT(SIGNAL,N); SIGNAL_COUNT<N*0.3".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 30.0)],
    });

    map.insert("holding_period".to_string(), FormulaTemplate {
        name: "持仓周期".to_string(),
        description: "持仓周期分析".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA5:=MA(CLOSE,5); MA20:=MA(CLOSE,20); HOLD_DAYS:=BARSLAST(CROSS(MA5,MA20)); HOLD_DAYS>5 AND HOLD_DAYS<20".to_string(),
        parameters: vec![],
    });

    map.insert("reentry_signal".to_string(), FormulaTemplate {
        name: "重新入场信号".to_string(),
        description: "离场后重新入场信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); EXIT:=CROSS(MA20,CLOSE); RECENT_EXIT:=EXIT OR REF(EXIT,1) OR REF(EXIT,2); RE_ENTRY:=CROSS(CLOSE,MA20); RECENT_EXIT AND RE_ENTRY".to_string(),
        parameters: vec![],
    });

    map.insert("partial_profit".to_string(), FormulaTemplate {
        name: "部分止盈".to_string(),
        description: "部分仓位止盈信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "PROFIT_PCT:=(CLOSE-REF(CLOSE,N))/REF(CLOSE,N)*100; PARTIAL_PROFIT:=PROFIT_PCT>10 AND PROFIT_PCT<20; VOL_CONFIRM:=VOLUME>MA(VOLUME,5); PARTIAL_PROFIT AND VOL_CONFIRM".to_string(),
        parameters: vec![("N".to_string(), 5.0, 30.0, 10.0)],
    });

    map.insert("add_position".to_string(), FormulaTemplate {
        name: "加仓信号".to_string(),
        description: "趋势确认加仓信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); TREND_UP:=CLOSE>MA20 AND MA20>REF(MA20,5); PULLBACK:=CLOSE<REF(CLOSE,1); SUPPORT_HOLD:=CLOSE>MA20*0.98; VOL_LOW:=VOLUME<MA(VOLUME,10)*0.8; TREND_UP AND PULLBACK AND SUPPORT_HOLD AND VOL_LOW".to_string(),
        parameters: vec![],
    });

    map.insert("reduce_position".to_string(), FormulaTemplate {
        name: "减仓信号".to_string(),
        description: "风险控制减仓信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); TREND_WEAK:=CLOSE<MA20*1.02; VOL_HIGH:=VOLUME>MA(VOLUME,10)*1.5; PROFIT_TAKEN:=(CLOSE-REF(CLOSE,10))/REF(CLOSE,10)*100>15; TREND_WEAK AND VOL_HIGH AND PROFIT_TAKEN".to_string(),
        parameters: vec![],
    });

    map.insert("hedge_signal".to_string(), FormulaTemplate {
        name: "对冲信号".to_string(),
        description: "对冲交易信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "BETA:=CORR(CLOSE,INDEX,N); MARKET_DOWN:=INDEX<MA(INDEX,20); STOCK_UP:=CLOSE>MA(CLOSE,20); DIVERGENCE:=BETA>0.5 AND MARKET_DOWN AND STOCK_UP; DIVERGENCE".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("arbitrage_signal".to_string(), FormulaTemplate {
        name: "套利信号".to_string(),
        description: "统计套利信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "SPREAD:=CLOSE-REF(CLOSE,1)*CORR(CLOSE,REF(CLOSE,1),N); MEAN_SPREAD:=MA(SPREAD,N); STD_SPREAD:=STD(SPREAD,N); Z_SCORE:=(SPREAD-MEAN_SPREAD)/STD_SPREAD; ABS(Z_SCORE)>2".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("sector_rotation".to_string(), FormulaTemplate {
        name: "板块轮动".to_string(),
        description: "板块轮动信号".to_string(),
        category: TemplateCategory::Strategy,
        source: "SECTOR_MA:=MA(CLOSE,20); MARKET_MA:=MA(INDEX,20); REL_STRENGTH:=SECTOR_MA/MARKET_MA; REL_STRENGTH>REF(REL_STRENGTH,5)*1.05".to_string(),
        parameters: vec![],
    });

    map.insert("momentum_factor".to_string(), FormulaTemplate {
        name: "动量因子".to_string(),
        description: "动量因子选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "MOM_12M:=(CLOSE-REF(CLOSE,250))/REF(CLOSE,250)*100; MOM_6M:=(CLOSE-REF(CLOSE,125))/REF(CLOSE,125)*100; MOM_3M:=(CLOSE-REF(CLOSE,60))/REF(CLOSE,60)*100; MOM_12M>0 AND MOM_6M>0 AND MOM_3M>0".to_string(),
        parameters: vec![],
    });

    map.insert("value_factor".to_string(), FormulaTemplate {
        name: "价值因子".to_string(),
        description: "价值因子选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "PE:=CLOSE/EPS; PB:=CLOSE/BVPS; PE<15 AND PB<1.5".to_string(),
        parameters: vec![],
    });

    map.insert("quality_factor".to_string(), FormulaTemplate {
        name: "质量因子".to_string(),
        description: "质量因子选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "ROE:=NET_INCOME/EQUITY; DEBT_RATIO:=TOTAL_DEBT/TOTAL_ASSETS; ROE>0.15 AND DEBT_RATIO<0.5".to_string(),
        parameters: vec![],
    });

    map.insert("low_vol_factor".to_string(), FormulaTemplate {
        name: "低波动因子".to_string(),
        description: "低波动因子选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "VOLATILITY:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N)*SQRT(252); VOLATILITY<0.3".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 30.0)],
    });

    map.insert("size_factor".to_string(), FormulaTemplate {
        name: "规模因子".to_string(),
        description: "小市值因子选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "MARKET_CAP:=CLOSE*TOTAL_SHARES; MARKET_CAP<5000000000".to_string(),
        parameters: vec![],
    });

    map.insert("dividend_yield".to_string(), FormulaTemplate {
        name: "股息率因子".to_string(),
        description: "高股息率选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "DIV_YIELD:=DIVIDEND/CLOSE*100; DIV_YIELD>3".to_string(),
        parameters: vec![],
    });

    map.insert("earnings_yield".to_string(), FormulaTemplate {
        name: "盈利收益率".to_string(),
        description: "盈利收益率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "EARNINGS_YIELD:=EPS/CLOSE*100; EARNINGS_YIELD>8".to_string(),
        parameters: vec![],
    });

    map.insert("book_yield".to_string(), FormulaTemplate {
        name: "账面收益率".to_string(),
        description: "账面收益率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "BOOK_YIELD:=BVPS/CLOSE*100; BOOK_YIELD>100".to_string(),
        parameters: vec![],
    });

    map.insert("cash_flow_yield".to_string(), FormulaTemplate {
        name: "现金流收益率".to_string(),
        description: "现金流收益率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "CF_YIELD:=CASH_FLOW/CLOSE*100; CF_YIELD>10".to_string(),
        parameters: vec![],
    });

    map.insert("ev_ebitda".to_string(), FormulaTemplate {
        name: "EV/EBITDA".to_string(),
        description: "企业价值倍数因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "EV:=MARKET_CAP+TOTAL_DEBT-CASH; EV_EBITDA:=EV/EBITDA; EV_EBITDA<10".to_string(),
        parameters: vec![],
    });

    map.insert("price_sales".to_string(), FormulaTemplate {
        name: "市销率".to_string(),
        description: "市销率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "PS:=CLOSE*TOTAL_SHARES/REVENUE; PS<2".to_string(),
        parameters: vec![],
    });

    map.insert("gross_margin".to_string(), FormulaTemplate {
        name: "毛利率因子".to_string(),
        description: "高毛利率选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "GROSS_MARGIN:=(REVENUE-COST)/REVENUE*100; GROSS_MARGIN>30".to_string(),
        parameters: vec![],
    });

    map.insert("net_margin".to_string(), FormulaTemplate {
        name: "净利率因子".to_string(),
        description: "高净利率选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "NET_MARGIN:=NET_INCOME/REVENUE*100; NET_MARGIN>10".to_string(),
        parameters: vec![],
    });

    map.insert("asset_turnover".to_string(), FormulaTemplate {
        name: "资产周转率".to_string(),
        description: "资产周转率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "ASSET_TURNOVER:=REVENUE/TOTAL_ASSETS; ASSET_TURNOVER>0.8".to_string(),
        parameters: vec![],
    });

    map.insert("inventory_turnover".to_string(), FormulaTemplate {
        name: "存货周转率".to_string(),
        description: "存货周转率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "INV_TURNOVER:=COST/INVENTORY; INV_TURNOVER>5".to_string(),
        parameters: vec![],
    });

    map.insert("current_ratio".to_string(), FormulaTemplate {
        name: "流动比率".to_string(),
        description: "流动比率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "CURRENT_RATIO:=CURRENT_ASSETS/CURRENT_LIABILITIES; CURRENT_RATIO>1.5".to_string(),
        parameters: vec![],
    });

    map.insert("quick_ratio".to_string(), FormulaTemplate {
        name: "速动比率".to_string(),
        description: "速动比率因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "QUICK_RATIO:=(CURRENT_ASSETS-INVENTORY)/CURRENT_LIABILITIES; QUICK_RATIO>1".to_string(),
        parameters: vec![],
    });

    map.insert("interest_coverage".to_string(), FormulaTemplate {
        name: "利息保障倍数".to_string(),
        description: "利息保障倍数因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "INTEREST_COVERAGE:=EBIT/INTEREST_EXPENSE; INTEREST_COVERAGE>5".to_string(),
        parameters: vec![],
    });

    map.insert("altman_z".to_string(), FormulaTemplate {
        name: "Altman Z值".to_string(),
        description: "Altman Z值破产预警".to_string(),
        category: TemplateCategory::Strategy,
        source: "Z:=1.2*WORKING_CAPITAL/TOTAL_ASSETS+1.4*RETAINED_EARNINGS/TOTAL_ASSETS+3.3*EBIT/TOTAL_ASSETS+0.6*MARKET_CAP/TOTAL_LIABILITIES+1.0*REVENUE/TOTAL_ASSETS; Z>3".to_string(),
        parameters: vec![],
    });

    map.insert("piotroski_f".to_string(), FormulaTemplate {
        name: "Piotroski F值".to_string(),
        description: "Piotroski F-Score评分".to_string(),
        category: TemplateCategory::Strategy,
        source: "F1:=IF(NET_INCOME>0,1,0); F2:=IF(OPERATING_CASH_FLOW>0,1,0); F3:=IF(ROA>REF(ROA,1),1,0); F4:=IF(OPERATING_CASH_FLOW>NET_INCOME,1,0); F_SCORE:=F1+F2+F3+F4; F_SCORE>=3".to_string(),
        parameters: vec![],
    });

    map.insert("graham_number".to_string(), FormulaTemplate {
        name: "格雷厄姆数值".to_string(),
        description: "格雷厄姆数值选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "GRAHAM:=SQRT(22.5*EPS*BVPS); CLOSE<GRAHAM".to_string(),
        parameters: vec![],
    });

    map.insert("magic_formula".to_string(), FormulaTemplate {
        name: "神奇公式".to_string(),
        description: "Joel Greenblatt神奇公式".to_string(),
        category: TemplateCategory::Strategy,
        source: "ROC:=EBIT/(WORKING_CAPITAL+NET_FIXED_ASSETS); EARNINGS_YIELD:=EBIT/ENTERPRISE_VALUE; ROC_RANK:=RANK(ROC); EY_RANK:=RANK(EARNINGS_YIELD); COMBINED_RANK:=ROC_RANK+EY_RANK; COMBINED_RANK<30".to_string(),
        parameters: vec![],
    });

    map.insert("nifty_fifty".to_string(), FormulaTemplate {
        name: "漂亮50策略".to_string(),
        description: "漂亮50成长股策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "GROWTH:=EPS/REF(EPS,4)-1; ROE:=NET_INCOME/EQUITY; PE:=CLOSE/EPS; GROWTH>0.2 AND ROE>0.15 AND PE<30".to_string(),
        parameters: vec![],
    });

    map.insert("dogs_of_dow".to_string(), FormulaTemplate {
        name: "狗股策略".to_string(),
        description: "道指狗股策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "DIV_YIELD:=DIVIDEND/CLOSE*100; HIGH_DIV:=DIV_YIELD>4; LARGE_CAP:=MARKET_CAP>10000000000; HIGH_DIV AND LARGE_CAP".to_string(),
        parameters: vec![],
    });

    map.insert("can_slim".to_string(), FormulaTemplate {
        name: "CAN SLIM策略".to_string(),
        description: "William O'Neil CAN SLIM策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "C_QTR:=EPS>REF(EPS,3)*1.25; A_ANNUAL:=EPS>REF(EPS,4)*1.25; N_NEW:=HIGH>HHV(HIGH,52); S_SUPPLY:=VOLUME/CAPITAL<0.1; L_LEADER:=CLOSE/MA(CLOSE,50)>1.3; I_INSTITUTION:=VOLUME>MA(VOLUME,20)*1.5; M_MARKET:=INDEX>MA(INDEX,50); C_QTR AND A_ANNUAL AND N_NEW".to_string(),
        parameters: vec![],
    });

    map.insert("dual_momentum".to_string(), FormulaTemplate {
        name: "双动量策略".to_string(),
        description: "Gary Antonacci双动量策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MOM_12:=CLOSE/REF(CLOSE,12)-1; MOM_6:=CLOSE/REF(CLOSE,6)-1; REL_MOM:=CLOSE/INDEX-1; MOM_12>0 AND MOM_6>0 AND REL_MOM>0".to_string(),
        parameters: vec![],
    });

    map.insert("trend_following".to_string(), FormulaTemplate {
        name: "趋势跟踪策略".to_string(),
        description: "经典趋势跟踪策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA_FAST:=MA(CLOSE,50); MA_SLOW:=MA(CLOSE,200); VOL_FILTER:=VOLUME>MA(VOLUME,50); CROSS(MA_FAST,MA_SLOW) AND VOL_FILTER".to_string(),
        parameters: vec![],
    });

    map.insert("mean_reversion".to_string(), FormulaTemplate {
        name: "均值回归策略".to_string(),
        description: "均值回归策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MA20:=MA(CLOSE,20); STD20:=STD(CLOSE,20); UPPER:=MA20+STD20*2; LOWER:=MA20-STD20*2; CLOSE<LOWER".to_string(),
        parameters: vec![],
    });

    map.insert("breakout_strategy_new".to_string(), FormulaTemplate {
        name: "突破策略增强".to_string(),
        description: "增强版突破策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "HH:=HHV(HIGH,N); LL:=LLV(LOW,N); RANGE:=HH-LL; BREAK_UP:=CLOSE>REF(HH,1); VOL_CONFIRM:=VOLUME>MA(VOLUME,20)*1.5; TREND_CONFIRM:=MA(CLOSE,10)>MA(CLOSE,30); BREAK_UP AND VOL_CONFIRM AND TREND_CONFIRM".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("reversal_strategy_new".to_string(), FormulaTemplate {
        name: "反转策略增强".to_string(),
        description: "增强版反转策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; OVERSOLD:=RSI<30; VOL_SHRINK:=VOLUME<MA(VOLUME,20)*0.5; BOUNCE:=CLOSE>REF(CLOSE,1); OVERSOLD AND VOL_SHRINK AND BOUNCE".to_string(),
        parameters: vec![],
    });

    map.insert("pair_ratio".to_string(), FormulaTemplate {
        name: "配对比率".to_string(),
        description: "配对交易比率分析".to_string(),
        category: TemplateCategory::Strategy,
        source: "RATIO:=CLOSE/REF(CLOSE,1); RATIO_MA:=MA(RATIO,N); RATIO_STD:=STD(RATIO,N); Z_SCORE:=(RATIO-RATIO_MA)/RATIO_STD; ABS(Z_SCORE)>2".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("correlation_trade".to_string(), FormulaTemplate {
        name: "相关性交易".to_string(),
        description: "基于相关性的交易".to_string(),
        category: TemplateCategory::Strategy,
        source: "CORR_VAL:=CORR(CLOSE,INDEX,N); DIVERGENCE:=CLOSE>MA(CLOSE,20) AND INDEX<MA(INDEX,20); CORR_VAL>0.7 AND DIVERGENCE".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("beta_neutral".to_string(), FormulaTemplate {
        name: "Beta中性".to_string(),
        description: "Beta中性策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "BETA:=COV(CLOSE,INDEX,N)/VAR(INDEX,N); BETA_HEDGE:=1-BETA; ABS(BETA_HEDGE)<0.2".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("alpha_generation".to_string(), FormulaTemplate {
        name: "Alpha生成".to_string(),
        description: "Alpha生成策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "STOCK_RET:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); MARKET_RET:=(INDEX-REF(INDEX,1))/REF(INDEX,1); ALPHA:=STOCK_RET-BETA*MARKET_RET; ALPHA>0.02".to_string(),
        parameters: vec![],
    });

    map.insert("multi_factor".to_string(), FormulaTemplate {
        name: "多因子模型".to_string(),
        description: "多因子综合选股".to_string(),
        category: TemplateCategory::Strategy,
        source: "MOM_FACTOR:=(CLOSE-REF(CLOSE,20))/REF(CLOSE,20); VALUE_FACTOR:=EPS/CLOSE; QUALITY_FACTOR:=ROE; MOM_SCORE:=RANK(MOM_FACTOR); VALUE_SCORE:=RANK(VALUE_FACTOR); QUALITY_SCORE:=RANK(QUALITY_FACTOR); COMPOSITE:=MOM_SCORE+VALUE_SCORE+QUALITY_SCORE; COMPOSITE>2".to_string(),
        parameters: vec![],
    });

    map.insert("risk_parity".to_string(), FormulaTemplate {
        name: "风险平价".to_string(),
        description: "风险平价配置".to_string(),
        category: TemplateCategory::Strategy,
        source: "VOL:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); RISK_CONTRIBUTION:=1/VOL; WEIGHT:=RISK_CONTRIBUTION/SUM(RISK_CONTRIBUTION,N); WEIGHT>0.1".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("min_variance".to_string(), FormulaTemplate {
        name: "最小方差".to_string(),
        description: "最小方差组合".to_string(),
        category: TemplateCategory::Strategy,
        source: "VAR_STOCK:=VAR((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); VAR_MARKET:=VAR((INDEX-REF(INDEX,1))/REF(INDEX,1),N); VAR_STOCK<VAR_MARKET*0.8".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("max_sharpe".to_string(), FormulaTemplate {
        name: "最大夏普".to_string(),
        description: "最大夏普比率组合".to_string(),
        category: TemplateCategory::Strategy,
        source: "RET:=MA((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); VOL:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),N); SHARPE:=RET/VOL; SHARPE>1".to_string(),
        parameters: vec![("N".to_string(), 10.0, 30.0, 20.0)],
    });

    map.insert("black_litterman".to_string(), FormulaTemplate {
        name: "Black-Litterman".to_string(),
        description: "Black-Litterman模型".to_string(),
        category: TemplateCategory::Strategy,
        source: "MARKET_WEIGHT:=MARKET_CAP/SUM(MARKET_CAP); VIEW_RETURN:=0.1; TAU:=0.05; POSTERIOR_WEIGHT:=MARKET_WEIGHT+TAU*VIEW_RETURN; POSTERIOR_WEIGHT>MARKET_WEIGHT*1.2".to_string(),
        parameters: vec![],
    });

    map.insert("factor_momentum".to_string(), FormulaTemplate {
        name: "因子动量".to_string(),
        description: "因子动量策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "MOM_1M:=(CLOSE-REF(CLOSE,20))/REF(CLOSE,20); MOM_3M:=(CLOSE-REF(CLOSE,60))/REF(CLOSE,60); MOM_6M:=(CLOSE-REF(CLOSE,120))/REF(CLOSE,120); FACTOR_MOM:=MOM_1M*0.5+MOM_3M*0.3+MOM_6M*0.2; FACTOR_MOM>0".to_string(),
        parameters: vec![],
    });

    map.insert("factor_value".to_string(), FormulaTemplate {
        name: "因子价值".to_string(),
        description: "价值因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "PE_RANK:=RANK(PE); PB_RANK:=RANK(PB); PS_RANK:=RANK(PS); VALUE_SCORE:=PE_RANK+PB_RANK+PS_RANK; VALUE_SCORE<30".to_string(),
        parameters: vec![],
    });

    map.insert("factor_quality".to_string(), FormulaTemplate {
        name: "因子质量".to_string(),
        description: "质量因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "ROE_RANK:=RANK(ROE); ROA_RANK:=RANK(ROA); GROSS_MARGIN_RANK:=RANK(GROSS_MARGIN); QUALITY_SCORE:=ROE_RANK+ROA_RANK+GROSS_MARGIN_RANK; QUALITY_SCORE>70".to_string(),
        parameters: vec![],
    });

    map.insert("factor_low_vol".to_string(), FormulaTemplate {
        name: "因子低波".to_string(),
        description: "低波动因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "VOL_20:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),20); VOL_60:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),60); VOL_RANK:=RANK(VOL_20+VOL_60); VOL_RANK<30".to_string(),
        parameters: vec![],
    });

    map.insert("factor_size".to_string(), FormulaTemplate {
        name: "因子规模".to_string(),
        description: "规模因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "CAP_RANK:=RANK(MARKET_CAP); SIZE_FACTOR:=CAP_RANK; SIZE_FACTOR<30".to_string(),
        parameters: vec![],
    });

    map.insert("factor_yield".to_string(), FormulaTemplate {
        name: "因子收益".to_string(),
        description: "收益率因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "DIV_YIELD_RANK:=RANK(DIVIDEND/CLOSE); EARNINGS_YIELD_RANK:=RANK(EPS/CLOSE); YIELD_SCORE:=DIV_YIELD_RANK+EARNINGS_YIELD_RANK; YIELD_SCORE>70".to_string(),
        parameters: vec![],
    });

    map.insert("factor_growth".to_string(), FormulaTemplate {
        name: "因子成长".to_string(),
        description: "成长因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "EPS_GROWTH:=(EPS-REF(EPS,4))/REF(EPS,4); REV_GROWTH:=(REVENUE-REF(REVENUE,4))/REF(REVENUE,4); GROWTH_SCORE:=RANK(EPS_GROWTH)+RANK(REV_GROWTH); GROWTH_SCORE>70".to_string(),
        parameters: vec![],
    });

    map.insert("sentiment_factor".to_string(), FormulaTemplate {
        name: "情绪因子".to_string(),
        description: "市场情绪因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "ADVANCE_DECLINE:=COUNT(CLOSE>REF(CLOSE,1),N)/N; VOL_RATIO:=VOLUME/MA(VOLUME,N); SENTIMENT:=ADVANCE_DECLINE*VOL_RATIO; SENTIMENT>1.2".to_string(),
        parameters: vec![("N".to_string(), 5.0, 20.0, 10.0)],
    });

    map.insert("liquidity_factor".to_string(), FormulaTemplate {
        name: "流动性因子".to_string(),
        description: "流动性因子策略".to_string(),
        category: TemplateCategory::Strategy,
        source: "TURNOVER:=VOLUME/CAPITAL; AMIHUD:=ABS((CLOSE-REF(CLOSE,1))/REF(CLOSE,1))/AMOUNT; LIQUIDITY_RANK:=RANK(TURNOVER)-RANK(AMIHUD); LIQUIDITY_RANK>50".to_string(),
        parameters: vec![],
    });

    map.insert("momentum_factor_new".to_string(), FormulaTemplate {
        name: "动量因子增强".to_string(),
        description: "增强版动量因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "MOM_3M:=(CLOSE-REF(CLOSE,60))/REF(CLOSE,60); MOM_6M:=(CLOSE-REF(CLOSE,120))/REF(CLOSE,120); MOM_12M:=(CLOSE-REF(CLOSE,240))/REF(CLOSE,240); MOM_COMPOSITE:=MOM_3M*0.5+MOM_6M*0.3+MOM_12M*0.2; MOM_RANK:=RANK(MOM_COMPOSITE); MOM_RANK>70".to_string(),
        parameters: vec![],
    });

    map.insert("reversal_factor".to_string(), FormulaTemplate {
        name: "反转因子".to_string(),
        description: "短期反转因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "SHORT_RET:=(CLOSE-REF(CLOSE,5))/REF(CLOSE,5); REVERSAL_RANK:=RANK(-SHORT_RET); REVERSAL_RANK>70".to_string(),
        parameters: vec![],
    });

    map.insert("technical_factor".to_string(), FormulaTemplate {
        name: "技术因子".to_string(),
        description: "技术分析因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "RSI:=SMA(MAX(CLOSE-REF(CLOSE,1),0),14,1)/SMA(ABS(CLOSE-REF(CLOSE,1)),14,1)*100; MACD_HIST:=MACD(12,26,9); TECH_SCORE:=RANK(RSI)+RANK(MACD_HIST); TECH_SCORE>100".to_string(),
        parameters: vec![],
    });

    map.insert("volume_factor".to_string(), FormulaTemplate {
        name: "成交量因子".to_string(),
        description: "成交量分析因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "VOL_RATIO:=VOLUME/MA(VOLUME,20); VOL_TREND:=MA(VOLUME,5)/MA(VOLUME,20); VOL_SCORE:=RANK(VOL_RATIO)+RANK(VOL_TREND); VOL_SCORE>100".to_string(),
        parameters: vec![],
    });

    map.insert("volatility_factor".to_string(), FormulaTemplate {
        name: "波动率因子".to_string(),
        description: "波动率分析因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "HIST_VOL:=STD((CLOSE-REF(CLOSE,1))/REF(CLOSE,1),20)*SQRT(252); IMPLIED_VOL:=VOLATILITY_SURFACE; VOL_RANK:=RANK(HIST_VOL); VOL_RANK<30".to_string(),
        parameters: vec![],
    });

    map.insert("skewness_factor".to_string(), FormulaTemplate {
        name: "偏度因子".to_string(),
        description: "收益分布偏度因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "RET:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); MEAN_RET:=MA(RET,N); STD_RET:=STD(RET,N); SKEW:=SUM((RET-MEAN_RET)^3,N)/(N*STD_RET^3); SKEW<-0.5".to_string(),
        parameters: vec![("N".to_string(), 20.0, 60.0, 30.0)],
    });

    map.insert("kurtosis_factor".to_string(), FormulaTemplate {
        name: "峰度因子".to_string(),
        description: "收益分布峰度因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "RET:=(CLOSE-REF(CLOSE,1))/REF(CLOSE,1); MEAN_RET:=MA(RET,N); STD_RET:=STD(RET,N); KURT:=SUM((RET-MEAN_RET)^4,N)/(N*STD_RET^4)-3; KURT>3".to_string(),
        parameters: vec![("N".to_string(), 20.0, 60.0, 30.0)],
    });

    map.insert("drawdown_factor".to_string(), FormulaTemplate {
        name: "回撤因子".to_string(),
        description: "最大回撤因子".to_string(),
        category: TemplateCategory::Strategy,
        source: "PEAK:=HHV(CLOSE,N); DRAWDOWN:=(PEAK-CLOSE)/PEAK; MAX_DD:=HHV(DRAWDOWN,N); DD_RANK:=RANK(-MAX_DD); DD_RANK>70".to_string(),
        parameters: vec![("N".to_string(), 20.0, 60.0, 30.0)],
    });

    map.insert("strat_ichimoku".to_string(), FormulaTemplate {
        name: "一目均衡策略".to_string(),
        description: "一目均衡表交叉".to_string(),
        category: TemplateCategory::Strategy,
        source: "CROSS(TENKAN,KIJUN)".to_string(),
        parameters: vec![],
    });

    map.insert("strat_supertrend".to_string(), FormulaTemplate {
        name: "超级趋势策略".to_string(),
        description: "超级趋势跟踪".to_string(),
        category: TemplateCategory::Strategy,
        source: "CLOSE>SUPERTREND(10,3)".to_string(),
        parameters: vec![],
    });

    map.insert("strat_vwap_revert".to_string(), FormulaTemplate {
        name: "VWAP回归".to_string(),
        description: "VWAP均值回归".to_string(),
        category: TemplateCategory::Strategy,
        source: "CLOSE<VWAP".to_string(),
        parameters: vec![],
    });

    map.insert("strat_keltner".to_string(), FormulaTemplate {
        name: "肯特纳通道".to_string(),
        description: "肯特纳突破".to_string(),
        category: TemplateCategory::Strategy,
        source: "CLOSE>KELTNER_UB".to_string(),
        parameters: vec![],
    });

    map.insert("strat_donchian".to_string(), FormulaTemplate {
        name: "唐奇安通道".to_string(),
        description: "唐奇安突破".to_string(),
        category: TemplateCategory::Strategy,
        source: "CLOSE=HHV(HIGH,20)".to_string(),
        parameters: vec![],
    });

    map.insert("strat_adx_trend".to_string(), FormulaTemplate {
        name: "ADX趋势".to_string(),
        description: "ADX强趋势过滤".to_string(),
        category: TemplateCategory::Strategy,
        source: "ADX(HIGH,LOW,CLOSE,14)>25".to_string(),
        parameters: vec![],
    });

    map.insert("strat_cci_overbought".to_string(), FormulaTemplate {
        name: "CCI超买".to_string(),
        description: "CCI超买超卖".to_string(),
        category: TemplateCategory::Strategy,
        source: "CCI(HIGH,LOW,CLOSE,14)>100".to_string(),
        parameters: vec![],
    });

    map.insert("strat_willr_extreme".to_string(), FormulaTemplate {
        name: "威廉极值".to_string(),
        description: "威廉指标极值".to_string(),
        category: TemplateCategory::Strategy,
        source: "WILLR(HIGH,LOW,CLOSE,14)<-80".to_string(),
        parameters: vec![],
    });

    map.insert("strat_mfi_divergence".to_string(), FormulaTemplate {
        name: "MFI背离".to_string(),
        description: "MFI超卖".to_string(),
        category: TemplateCategory::Strategy,
        source: "MFI(HIGH,LOW,CLOSE,VOLUME,14)<20".to_string(),
        parameters: vec![],
    });

    map.insert("strat_elder_ray".to_string(), FormulaTemplate {
        name: "老鹰射线".to_string(),
        description: "老鹰射线多头".to_string(),
        category: TemplateCategory::Strategy,
        source: "ELDER_RAY_BULL(CLOSE,13)>0".to_string(),
        parameters: vec![],
    });

    map.insert("strat_chaikin_vol".to_string(), FormulaTemplate {
        name: "佳庆波动".to_string(),
        description: "佳庆波动扩张".to_string(),
        category: TemplateCategory::Strategy,
        source: "CHAIKIN_VOL(HIGH,LOW,CLOSE,10,20)>0".to_string(),
        parameters: vec![],
    });

    map.insert("strat_force_index".to_string(), FormulaTemplate {
        name: "力度指数".to_string(),
        description: "力度指数多头".to_string(),
        category: TemplateCategory::Strategy,
        source: "FORCE_INDEX(CLOSE,VOLUME,13)>0".to_string(),
        parameters: vec![],
    });

    map.insert("strat_mass_index".to_string(), FormulaTemplate {
        name: "质量指数".to_string(),
        description: "质量指数反转".to_string(),
        category: TemplateCategory::Strategy,
        source: "MASS_INDEX(HIGH,LOW,25)>27".to_string(),
        parameters: vec![],
    });

    map.insert("strat_squeeze".to_string(), FormulaTemplate {
        name: "挤压动量".to_string(),
        description: "TTM挤压".to_string(),
        category: TemplateCategory::Strategy,
        source: "SQUEEZE_MOMENTUM(CLOSE,20,2,1.5)>0".to_string(),
        parameters: vec![],
    });

    map.insert("strat_stc".to_string(), FormulaTemplate {
        name: "Schaff趋势周期".to_string(),
        description: "STC超买".to_string(),
        category: TemplateCategory::Strategy,
        source: "STC(CLOSE,23,50,25)>80".to_string(),
        parameters: vec![],
    });

    map.insert("strat_coppock".to_string(), FormulaTemplate {
        name: "估波指标".to_string(),
        description: "估波指标底部".to_string(),
        category: TemplateCategory::Strategy,
        source: "COPPOCK(CLOSE,14,11,10)>0".to_string(),
        parameters: vec![],
    });

    map.insert("strat_ultimate_osc".to_string(), FormulaTemplate {
        name: "终极振荡".to_string(),
        description: "终极振荡超买".to_string(),
        category: TemplateCategory::Strategy,
        source: "ULT_OSC(HIGH,LOW,CLOSE,7,14,28)>70".to_string(),
        parameters: vec![],
    });

    map.insert("strat_vortex".to_string(), FormulaTemplate {
        name: "涡旋指标".to_string(),
        description: "涡旋交叉".to_string(),
        category: TemplateCategory::Strategy,
        source: "CROSS(VI_PLUS(HIGH,LOW,CLOSE,14),VI_MINUS(HIGH,LOW,CLOSE,14))".to_string(),
        parameters: vec![],
    });

    map.insert("strat_choppiness".to_string(), FormulaTemplate {
        name: "趋势强度".to_string(),
        description: "趋势强度判断".to_string(),
        category: TemplateCategory::Strategy,
        source: "CHOP(HIGH,LOW,CLOSE,14)<61.8".to_string(),
        parameters: vec![],
    });

    map.insert("strat_fisher".to_string(), FormulaTemplate {
        name: "费舍尔变换".to_string(),
        description: "费舍尔变换多头".to_string(),
        category: TemplateCategory::Strategy,
        source: "FISHER(HIGH,LOW,9)>0".to_string(),
        parameters: vec![],
    });

    map.insert("strat_tsi".to_string(), FormulaTemplate {
        name: "真实强度".to_string(),
        description: "真实强度指数多头".to_string(),
        category: TemplateCategory::Strategy,
        source: "TSI(CLOSE,25,13)>0".to_string(),
        parameters: vec![],
    });

    // ========== 飞狐交易师（FoxTrader）策略模板 ==========
    map.insert("fox_ma_cross".to_string(), FormulaTemplate {
        name: "均线交叉策略".to_string(),
        description: "飞狐交易师均线交叉策略，短期均线上穿长期均线买入".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "MA5:=MA(CLOSE,5); MA10:=MA(CLOSE,10); MA20:=MA(CLOSE,20); BUY_COND:=CROSS(MA5,MA10); SELL_COND:=CROSS(MA10,MA5); FOX_TRADE_SIGNAL(BUY_COND,SELL_COND)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_macd_golden".to_string(), FormulaTemplate {
        name: "MACD金叉策略".to_string(),
        description: "飞狐交易师MACD金叉策略，DIF上穿DEA买入".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA1:=EMA(DIF,9); MACD1:=(DIF-DEA1)*2; BUY_COND:=CROSS(DIF,DEA1); SELL_COND:=CROSS(DEA1,DIF); FOX_BACKTEST(BUY_COND,SELL_COND,CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_rsi_overtrade".to_string(), FormulaTemplate {
        name: "RSI超买超卖策略".to_string(),
        description: "飞狐交易师RSI超买超卖策略，RSI低位买入高位卖出".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "RSI14:=RSI(CLOSE,14); BUY_COND:=CROSS(30,RSI14); SELL_COND:=CROSS(RSI14,70); FOX_BACKTEST(BUY_COND,SELL_COND,CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_boll_break".to_string(), FormulaTemplate {
        name: "布林带突破策略".to_string(),
        description: "飞狐交易师布林带突破策略，突破下轨买入突破上轨卖出".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "MID:=MA(CLOSE,20); UPPER:=MID+2*STD(CLOSE,20); LOWER:=MID-2*STD(CLOSE,20); BUY_COND:=CROSS(LOWER,CLOSE); SELL_COND:=CROSS(CLOSE,UPPER); FOX_BACKTEST(BUY_COND,SELL_COND,CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_turtle".to_string(), FormulaTemplate {
        name: "海龟交易策略".to_string(),
        description: "飞狐交易师海龟交易策略，突破N日新高买入跌破N日新低卖出".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "HH:=HHV(HIGH,N); LL:=LLV(LOW,N); BUY_COND:=CROSS(CLOSE,REF(HH,1)); SELL_COND:=CROSS(REF(LL,1),CLOSE); FOX_BACKTEST(BUY_COND,SELL_COND,CLOSE)".to_string(),
        parameters: vec![("N".to_string(), 10.0, 60.0, 20.0)],
    });

    map.insert("fox_kdj_strategy".to_string(), FormulaTemplate {
        name: "KDJ策略".to_string(),
        description: "KDJ金叉死叉".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(CROSS(K,D),CLOSE);FOX_SELL(CROSS(D,K),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_boll_mean".to_string(), FormulaTemplate {
        name: "布林回归".to_string(),
        description: "布林带均值回归".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(CLOSE<BOLL_LB,CLOSE);FOX_SELL(CLOSE>BOLL_UB,CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_momentum".to_string(), FormulaTemplate {
        name: "动量策略".to_string(),
        description: "动量突破".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(ROC(CLOSE,10)>0,CLOSE);FOX_SELL(ROC(CLOSE,10)<0,CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_volume_break".to_string(), FormulaTemplate {
        name: "放量突破".to_string(),
        description: "放量突破".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(VOLUME>MA(VOLUME,20)*2 AND CLOSE>MA(CLOSE,20),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_double_bottom".to_string(), FormulaTemplate {
        name: "双底策略".to_string(),
        description: "双底形态".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(LOW=LLV(LOW,20) AND REF(LOW,1)=LLV(LOW,20),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_trend_follow".to_string(), FormulaTemplate {
        name: "趋势跟踪".to_string(),
        description: "均线趋势跟踪".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(CLOSE>MA(CLOSE,60),CLOSE);FOX_SELL(CLOSE<MA(CLOSE,60),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_mean_revert".to_string(), FormulaTemplate {
        name: "均值回归".to_string(),
        description: "均值回归买入".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(CLOSE<MA(CLOSE,20)-2*STD(CLOSE,20),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_breakout".to_string(), FormulaTemplate {
        name: "突破策略".to_string(),
        description: "N日突破".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(CLOSE=HHV(CLOSE,20),CLOSE);FOX_SELL(CLOSE=LLV(CLOSE,20),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_scalping".to_string(), FormulaTemplate {
        name: "日内短线".to_string(),
        description: "短线交易".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(CROSS(MA(C,5),MA(C,10)) AND VOLUME>REF(VOLUME,1),CLOSE)".to_string(),
        parameters: vec![],
    });

    map.insert("fox_swing".to_string(), FormulaTemplate {
        name: "波段交易".to_string(),
        description: "波段交易".to_string(),
        category: TemplateCategory::FoxTrader,
        source: "FOX_BUY(FOX_ZIG(1,5)>REF(FOX_ZIG(1,5),1),CLOSE)".to_string(),
        parameters: vec![],
    });

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_new_has_builtin() {
        let lib = FormulaTemplates::new();
        assert!(lib.list_all().len() >= 50);
    }

    #[test]
    fn test_get_template_by_name() {
        let lib = FormulaTemplates::new();
        let tmpl = lib.get("ma_cross");
        assert!(tmpl.is_some());
        assert_eq!(tmpl.unwrap().name, "均线金叉死叉");
    }

    #[test]
    fn test_get_nonexistent_template() {
        let lib = FormulaTemplates::new();
        assert!(lib.get("nonexistent").is_none());
    }

    #[test]
    fn test_get_by_category_moving_average() {
        let lib = FormulaTemplates::new();
        let ma_templates = lib.get_by_category(&TemplateCategory::MovingAverage);
        assert!(ma_templates.len() >= 5);
    }

    #[test]
    fn test_get_by_category_trend() {
        let lib = FormulaTemplates::new();
        let trend_templates = lib.get_by_category(&TemplateCategory::Trend);
        assert!(trend_templates.len() >= 3);
    }

    #[test]
    fn test_get_by_category_oscillator() {
        let lib = FormulaTemplates::new();
        let osc_templates = lib.get_by_category(&TemplateCategory::Oscillator);
        assert!(osc_templates.len() >= 5);
    }

    #[test]
    fn test_get_by_category_volume() {
        let lib = FormulaTemplates::new();
        let vol_templates = lib.get_by_category(&TemplateCategory::Volume);
        assert!(vol_templates.len() >= 5);
    }

    #[test]
    fn test_get_by_category_strategy() {
        let lib = FormulaTemplates::new();
        let strat_templates = lib.get_by_category(&TemplateCategory::Strategy);
        assert!(strat_templates.len() >= 5);
    }

    #[test]
    fn test_get_by_category_classic() {
        let lib = FormulaTemplates::new();
        let classic_templates = lib.get_by_category(&TemplateCategory::Classic);
        assert!(classic_templates.len() >= 5);
    }

    #[test]
    fn test_search_by_keyword() {
        let lib = FormulaTemplates::new();
        let results = lib.search("MACD");
        assert!(!results.is_empty());
        for r in &results {
            assert!(
                r.name.to_lowercase().contains("macd")
                    || r.description.to_lowercase().contains("macd")
                    || r.source.to_lowercase().contains("macd")
            );
        }
    }

    #[test]
    fn test_search_by_chinese_keyword() {
        let lib = FormulaTemplates::new();
        let results = lib.search("超买");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_no_results() {
        let lib = FormulaTemplates::new();
        let results = lib.search("zzzzznotfound");
        assert!(results.is_empty());
    }

    #[test]
    fn test_list_all() {
        let lib = FormulaTemplates::new();
        let all = lib.list_all();
        assert!(all.len() >= 50);
    }

    #[test]
    fn test_categories_list() {
        let cats = FormulaTemplates::categories();
        assert!(cats.len() >= 7);
    }

    #[test]
    fn test_template_has_parameters() {
        let lib = FormulaTemplates::new();
        let tmpl = lib.get("ma_cross").unwrap();
        assert_eq!(tmpl.parameters.len(), 2);
        assert_eq!(tmpl.parameters[0].0, "SHORT");
        assert!((tmpl.parameters[0].3 - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_template_category_assignment() {
        let lib = FormulaTemplates::new();
        let tmpl = lib.get("kdj_golden_cross").unwrap();
        assert_eq!(tmpl.category, TemplateCategory::Oscillator);
    }

    #[test]
    fn test_template_source_not_empty() {
        let lib = FormulaTemplates::new();
        for tmpl in lib.list_all() {
            assert!(
                !tmpl.source.is_empty(),
                "Template '{}' has empty source",
                tmpl.name
            );
        }
    }

    #[test]
    fn test_template_name_not_empty() {
        let lib = FormulaTemplates::new();
        for tmpl in lib.list_all() {
            assert!(!tmpl.name.is_empty(), "Template has empty name");
        }
    }

    #[test]
    fn test_search_case_insensitive() {
        let lib = FormulaTemplates::new();
        let r1 = lib.search("macd");
        let r2 = lib.search("MACD");
        assert_eq!(r1.len(), r2.len());
    }

    #[test]
    fn test_all_categories_have_templates() {
        let lib = FormulaTemplates::new();
        for cat in FormulaTemplates::categories() {
            let templates = lib.get_by_category(&cat);
            assert!(!templates.is_empty(), "Category {:?} has no templates", cat);
        }
    }
}
