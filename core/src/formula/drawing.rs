use ndarray::Array1;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum DrawCommand {
    Text {
        #[cfg_attr(feature = "serde", serde(skip))]
        condition: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price: Array1<f64>,
        text: String,
        color: String,
    },
    Icon {
        #[cfg_attr(feature = "serde", serde(skip))]
        condition: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(rename = "iconType"))]
        icon_type: i32,
        color: String,
    },
    StickLine {
        #[cfg_attr(feature = "serde", serde(skip))]
        condition: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price2: Array1<f64>,
        width: i32,
        empty: bool,
        color: String,
    },
    Line {
        #[cfg_attr(feature = "serde", serde(skip))]
        cond1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        cond2: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price2: Array1<f64>,
        expand: i32,
        color: String,
    },
    Band {
        #[cfg_attr(feature = "serde", serde(skip))]
        val1: Array1<f64>,
        color1: String,
        #[cfg_attr(feature = "serde", serde(skip))]
        val2: Array1<f64>,
        color2: String,
    },
    KLine {
        #[cfg_attr(feature = "serde", serde(skip))]
        open: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        high: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        low: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        close: Array1<f64>,
    },
    Rect {
        #[cfg_attr(feature = "serde", serde(skip))]
        x1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        y1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        x2: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        y2: Array1<f64>,
        color: String,
    },
    FillRgn {
        #[cfg_attr(feature = "serde", serde(skip))]
        cond: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price2: Array1<f64>,
        color: String,
    },
    PartLine {
        #[cfg_attr(feature = "serde", serde(skip))]
        cond: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price: Array1<f64>,
        color: String,
    },
    PolyLine {
        #[cfg_attr(feature = "serde", serde(skip))]
        cond: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price: Array1<f64>,
        color: String,
    },
    Background {
        #[cfg_attr(feature = "serde", serde(skip))]
        cond: Array1<f64>,
        color: String,
    },
    SlopeLine {
        #[cfg_attr(feature = "serde", serde(skip))]
        cond1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price1: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        cond2: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price2: Array1<f64>,
        color: String,
    },
    TextFix {
        x: f64,
        y: f64,
        text: String,
        color: String,
    },
    Number {
        #[cfg_attr(feature = "serde", serde(skip))]
        condition: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        price: Array1<f64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        number: Array1<f64>,
        precision: i32,
        color: String,
    },
    VertLine {
        #[cfg_attr(feature = "serde", serde(skip))]
        condition: Array1<f64>,
        color: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DrawResult {
    pub commands: Vec<DrawCommand>,
}

impl DrawResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_text(&mut self, cond: Array1<f64>, price: Array1<f64>, text: String, color: String) {
        self.commands.push(DrawCommand::Text {
            condition: cond,
            price,
            text,
            color,
        });
    }

    pub fn add_icon(
        &mut self,
        cond: Array1<f64>,
        price: Array1<f64>,
        icon_type: i32,
        color: String,
    ) {
        self.commands.push(DrawCommand::Icon {
            condition: cond,
            price,
            icon_type,
            color,
        });
    }

    pub fn add_stick(
        &mut self,
        cond: Array1<f64>,
        p1: Array1<f64>,
        p2: Array1<f64>,
        width: i32,
        empty: bool,
        color: String,
    ) {
        self.commands.push(DrawCommand::StickLine {
            condition: cond,
            price1: p1,
            price2: p2,
            width,
            empty,
            color,
        });
    }

    pub fn add_line(
        &mut self,
        cond1: Array1<f64>,
        price1: Array1<f64>,
        cond2: Array1<f64>,
        price2: Array1<f64>,
        expand: i32,
        color: String,
    ) {
        self.commands.push(DrawCommand::Line {
            cond1, price1, cond2, price2, expand, color,
        });
    }

    pub fn add_band(&mut self, val1: Array1<f64>, color1: String, val2: Array1<f64>, color2: String) {
        self.commands.push(DrawCommand::Band { val1, color1, val2, color2 });
    }

    pub fn add_kline(&mut self, open: Array1<f64>, high: Array1<f64>, low: Array1<f64>, close: Array1<f64>) {
        self.commands.push(DrawCommand::KLine { open, high, low, close });
    }

    pub fn add_rect(&mut self, x1: Array1<f64>, y1: Array1<f64>, x2: Array1<f64>, y2: Array1<f64>, color: String) {
        self.commands.push(DrawCommand::Rect { x1, y1, x2, y2, color });
    }

    pub fn add_fill_rgn(&mut self, cond: Array1<f64>, price1: Array1<f64>, price2: Array1<f64>, color: String) {
        self.commands.push(DrawCommand::FillRgn { cond, price1, price2, color });
    }

    pub fn add_part_line(&mut self, cond: Array1<f64>, price: Array1<f64>, color: String) {
        self.commands.push(DrawCommand::PartLine { cond, price, color });
    }

    pub fn add_poly_line(&mut self, cond: Array1<f64>, price: Array1<f64>, color: String) {
        self.commands.push(DrawCommand::PolyLine { cond, price, color });
    }

    pub fn add_background(&mut self, cond: Array1<f64>, color: String) {
        self.commands.push(DrawCommand::Background { cond, color });
    }

    pub fn add_slope_line(
        &mut self,
        cond1: Array1<f64>,
        price1: Array1<f64>,
        cond2: Array1<f64>,
        price2: Array1<f64>,
        color: String,
    ) {
        self.commands.push(DrawCommand::SlopeLine { cond1, price1, cond2, price2, color });
    }

    pub fn add_text_fix(&mut self, x: f64, y: f64, text: String, color: String) {
        self.commands.push(DrawCommand::TextFix { x, y, text, color });
    }

    pub fn add_number(
        &mut self,
        cond: Array1<f64>,
        price: Array1<f64>,
        number: Array1<f64>,
        precision: i32,
        color: String,
    ) {
        self.commands.push(DrawCommand::Number { condition: cond, price, number, precision, color });
    }

    pub fn add_vert_line(&mut self, cond: Array1<f64>, color: String) {
        self.commands.push(DrawCommand::VertLine { condition: cond, color });
    }
}
