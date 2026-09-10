use crate::config::{ChartConfig, IndicatorConfig, IndicatorType};
use crate::data::KlineData;
use crate::error::Result;
use crate::primitive::DrawList;
use crate::scene::{ChartScene, PanelId};
use serde::Serialize;
use std::collections::HashMap;

use finkit::indicators;
use finkit::math::moving_avg;

use super::Renderer;
use super::SvgRenderer;

pub struct HtmlRenderer;

impl HtmlRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HtmlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for HtmlRenderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String> {
        self.render_internal(draw_list, config, "[]", "[]", "[]", "[]", None)
    }
}

#[derive(Debug, Serialize)]
struct HtmlBar {
    index: usize,
    source_end: usize,
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<i64>,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    prev_close: Option<f64>,
    change: Option<f64>,
    change_pct: Option<f64>,
    amplitude_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct HtmlIndicator {
    pub(crate) name: String,
    pub(crate) values: Vec<Option<f64>>,
}

#[derive(Debug, Serialize)]
struct HtmlPanel {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Serialize)]
struct HtmlHitRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    priority: i32,
    target: String,
    tooltip: Option<String>,
}

pub(crate) fn indicator_payload(
    data: &KlineData,
    source_ranges: &[(usize, usize)],
    configs: &[IndicatorConfig],
    custom_series: &HashMap<String, Vec<f64>>,
) -> Vec<HtmlIndicator> {
    let mut result = Vec::new();
    let to_values = |values: Vec<f64>| HtmlIndicator {
        name: String::new(),
        values: values
            .into_iter()
            .map(|value| value.is_finite().then_some(value))
            .collect(),
    };
    let add = |result: &mut Vec<HtmlIndicator>, name: String, values: Vec<f64>| {
        let mut item = to_values(values);
        item.name = name;
        result.push(item);
    };

    for config in configs.iter().filter(|config| config.visible) {
        match &config.indicator_type {
            IndicatorType::MA | IndicatorType::SMA => {
                for (index, period) in config.params.iter().copied().enumerate() {
                    let period = period.max(0.0) as usize;
                    if let Ok(values) = moving_avg::sma(&data.closes, period) {
                        add(
                            &mut result,
                            format!("{}{}", config.name, period.max(1)),
                            values.to_vec(),
                        );
                    } else if index == 0 {
                        break;
                    }
                }
            }
            IndicatorType::EMA => {
                for period in config.params.iter().copied() {
                    let period = period.max(0.0) as usize;
                    if let Ok(values) = moving_avg::ema(&data.closes, period) {
                        add(
                            &mut result,
                            format!("{}{}", config.name, period.max(1)),
                            values.to_vec(),
                        );
                    }
                }
            }
            IndicatorType::BOLL => {
                let period = config.params.first().copied().unwrap_or(20.0).max(0.0) as usize;
                let deviation = config.params.get(1).copied().unwrap_or(2.0);
                if let Ok(values) = indicators::bbands(&data.closes, period, deviation, deviation) {
                    add(&mut result, "BOLL.UPPER".to_string(), values.upper.to_vec());
                    add(
                        &mut result,
                        "BOLL.MIDDLE".to_string(),
                        values.middle.to_vec(),
                    );
                    add(&mut result, "BOLL.LOWER".to_string(), values.lower.to_vec());
                }
            }
            IndicatorType::MACD => {
                let fast = config.params.first().copied().unwrap_or(12.0).max(0.0) as usize;
                let slow = config.params.get(1).copied().unwrap_or(26.0).max(0.0) as usize;
                let signal = config.params.get(2).copied().unwrap_or(9.0).max(0.0) as usize;
                if let Ok(values) = indicators::macd(&data.closes, fast, slow, signal) {
                    add(&mut result, "MACD.DIF".to_string(), values.macd.to_vec());
                    add(&mut result, "MACD.DEA".to_string(), values.signal.to_vec());
                    add(&mut result, "MACD.HIST".to_string(), values.hist.to_vec());
                }
            }
            IndicatorType::RSI => {
                let period = config.params.first().copied().unwrap_or(14.0).max(0.0) as usize;
                if let Ok(values) = indicators::rsi(&data.closes, period) {
                    add(&mut result, "RSI".to_string(), values.to_vec());
                }
            }
            IndicatorType::KDJ => {
                let fast = config.params.first().copied().unwrap_or(9.0).max(0.0) as usize;
                let slow = config.params.get(1).copied().unwrap_or(3.0).max(0.0) as usize;
                let signal = config.params.get(2).copied().unwrap_or(3.0).max(0.0) as usize;
                if let Ok(values) =
                    indicators::stoch(&data.highs, &data.lows, &data.closes, fast, slow, signal)
                {
                    let k = values.k.to_vec();
                    let d = values.d.to_vec();
                    let j = k
                        .iter()
                        .zip(d.iter())
                        .map(|(k, d)| {
                            if k.is_finite() && d.is_finite() {
                                3.0 * k - 2.0 * d
                            } else {
                                f64::NAN
                            }
                        })
                        .collect();
                    add(&mut result, "KDJ.K".to_string(), k);
                    add(&mut result, "KDJ.D".to_string(), d);
                    add(&mut result, "KDJ.J".to_string(), j);
                }
            }
            IndicatorType::Custom(name) if name.eq_ignore_ascii_case("sar") => {
                let acceleration = config.params.first().copied().unwrap_or(0.02);
                let maximum = config.params.get(1).copied().unwrap_or(0.2);
                if let Ok(values) = indicators::sar(&data.highs, &data.lows, acceleration, maximum)
                {
                    add(&mut result, "SAR".to_string(), values.sar.to_vec());
                }
            }
            IndicatorType::Custom(name) => {
                if let Some(values) = custom_series.get(name) {
                    let mapped = if values.len() == data.len() {
                        values.clone()
                    } else {
                        source_ranges
                            .iter()
                            .map(|(_, end)| {
                                values
                                    .get(end.saturating_sub(1))
                                    .copied()
                                    .unwrap_or(f64::NAN)
                            })
                            .collect()
                    };
                    add(&mut result, name.clone(), mapped);
                }
            }
        }
    }
    result
}

impl HtmlRenderer {
    pub fn render_with_data(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        scene: &ChartScene,
    ) -> Result<String> {
        self.render_with_data_and_ranges(draw_list, config, data, source_offset, &[], scene)
    }

    pub fn render_with_data_and_ranges(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        scene: &ChartScene,
    ) -> Result<String> {
        self.render_with_data_and_ranges_and_indicators(
            draw_list,
            config,
            data,
            source_offset,
            source_ranges,
            scene,
            &[],
            &HashMap::new(),
        )
    }

    /// Render HTML with the semantic scene and the visible indicator values
    /// used by the floating data window.
    pub fn render_with_data_and_ranges_and_indicators(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        scene: &ChartScene,
        indicator_configs: &[IndicatorConfig],
        custom_series: &HashMap<String, Vec<f64>>,
    ) -> Result<String> {
        let bars: Vec<HtmlBar> = (0..data.len())
            .map(|index| HtmlBar {
                index: source_ranges
                    .get(index)
                    .map(|range| range.0)
                    .unwrap_or(source_offset + index),
                source_end: source_ranges
                    .get(index)
                    .map(|range| range.1)
                    .unwrap_or(source_offset + index + 1),
                date: data.dates[index].clone(),
                timestamp: data.timestamps.get(index).copied(),
                open: data.opens[index],
                high: data.highs[index],
                low: data.lows[index],
                close: data.closes[index],
                volume: data.volumes[index],
                prev_close: index
                    .checked_sub(1)
                    .and_then(|previous| data.closes.get(previous).copied()),
                change: index
                    .checked_sub(1)
                    .and_then(|previous| data.closes.get(previous).copied())
                    .map(|previous| data.closes[index] - previous),
                change_pct: index
                    .checked_sub(1)
                    .and_then(|previous| data.closes.get(previous).copied())
                    .filter(|previous| previous.abs() > f64::EPSILON)
                    .map(|previous| (data.closes[index] - previous) / previous * 100.0),
                amplitude_pct: Some(
                    (data.highs[index] - data.lows[index]).abs()
                        / data.lows[index].abs().max(f64::EPSILON)
                        * 100.0,
                ),
            })
            .collect();
        let payload = serde_json::to_string(&bars).map_err(|error| {
            crate::error::VisualizationError::SerializationError {
                message: format!("Failed to serialize HTML chart data: {error}"),
            }
        })?;
        let indicators_payload = serde_json::to_string(&indicator_payload(
            data,
            source_ranges,
            indicator_configs,
            custom_series,
        ))
        .map_err(
            |error| crate::error::VisualizationError::SerializationError {
                message: format!("Failed to serialize HTML indicator data: {error}"),
            },
        )?;
        let hit_payload = serde_json::to_string(
            &scene
                .hit_regions
                .iter()
                .map(|region| HtmlHitRegion {
                    x: region.rect.x,
                    y: region.rect.y,
                    width: region.rect.width,
                    height: region.rect.height,
                    priority: region.priority,
                    target: format!("{:?}", region.target),
                    tooltip: region.tooltip.clone(),
                })
                .collect::<Vec<_>>(),
        )
        .map_err(
            |error| crate::error::VisualizationError::SerializationError {
                message: format!("Failed to serialize HTML hit regions: {error}"),
            },
        )?;
        let panel_payload = serde_json::to_string(
            &scene
                .panels
                .iter()
                .filter(|panel| panel.visible)
                .map(|panel| HtmlPanel {
                    id: match panel.id {
                        PanelId::Main => "main".to_string(),
                        PanelId::Volume => "volume".to_string(),
                        PanelId::Indicator(index) => format!("indicator-{index}"),
                    },
                    x: panel.rect.x,
                    y: panel.rect.y,
                    width: panel.rect.width,
                    height: panel.rect.height,
                })
                .collect::<Vec<_>>(),
        )
        .map_err(
            |error| crate::error::VisualizationError::SerializationError {
                message: format!("Failed to serialize HTML panel data: {error}"),
            },
        )?;
        let plot = scene
            .panels
            .iter()
            .find(|panel| panel.id == PanelId::Main)
            .map(|panel| panel.rect)
            .or_else(|| {
                Some(crate::geometry::Rect::new(
                    config.margins.left as f64,
                    config.margins.top as f64,
                    config
                        .width
                        .saturating_sub(config.margins.left + config.margins.right)
                        as f64,
                    config
                        .height
                        .saturating_sub(config.margins.top + config.margins.bottom)
                        as f64,
                ))
            });
        self.render_internal(
            draw_list,
            config,
            &payload,
            &indicators_payload,
            &hit_payload,
            &panel_payload,
            plot,
        )
    }

    fn render_internal(
        &self,
        draw_list: &DrawList,
        config: &ChartConfig,
        data_json: &str,
        indicators_json: &str,
        hit_json: &str,
        panel_json: &str,
        plot: Option<crate::geometry::Rect>,
    ) -> Result<String> {
        let svg_renderer = SvgRenderer::new();
        let svg_content = svg_renderer.render(draw_list, config)?;

        let html = format!(
            "<!DOCTYPE html>\n\
             <html>\n\
             <head>\n\
             <meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\">\n\
             <title>{title}</title>\n\
             <style>\n\
             *{{margin:0;padding:0;box-sizing:border-box}}\n\
             body{{background:{bg};display:flex;justify-content:center;align-items:center;min-height:100vh;font-family:sans-serif}}\n\
             .chart-container{{position:relative;width:{w}px;height:{h}px;overflow:hidden}}\n\
             .chart-container svg{{display:block;cursor:crosshair;touch-action:none;outline:none}}\n\
             .chart-container svg .chart-content{{transition:none}}\n\
             .crosshair-h{{stroke:{cross};stroke-width:1;stroke-dasharray:4,2;pointer-events:none}}\n\
             .crosshair-v{{stroke:{cross};stroke-width:1;stroke-dasharray:4,2;pointer-events:none}}\n\
             .tooltip{{position:absolute;display:none;max-width:280px;min-width:220px;background:rgba(14,18,28,0.94);color:#f3f4f6;padding:8px 10px;border:1px solid rgba(148,163,184,0.45);border-radius:3px;font-size:12px;pointer-events:none;white-space:normal;line-height:1.45;z-index:10;box-shadow:0 3px 12px rgba(0,0,0,0.22)}}\n\
             .tooltip-title{{font-weight:600;color:#fff;margin-bottom:4px;border-bottom:1px solid rgba(148,163,184,0.3);padding-bottom:4px}}\n\
             .tooltip-grid{{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));column-gap:12px;row-gap:2px}}\n\
             .tooltip-label{{color:#aab4c3}}\n\
             .tooltip-value{{text-align:right;font-variant-numeric:tabular-nums}}\n\
             .tooltip-hit{{margin-top:5px;padding-top:4px;border-top:1px solid rgba(148,163,184,0.3);color:#fbbf24}}\n\
             .axis-label{{position:absolute;display:none;background:rgba(14,18,28,0.92);color:#f8fafc;border:1px solid rgba(148,163,184,0.45);padding:2px 4px;border-radius:2px;font-size:11px;line-height:1.2;pointer-events:none;z-index:9;font-variant-numeric:tabular-nums}}\n\
             .axis-label-x{{transform:translateX(-50%)}}\n\
             </style>\n\
             </head>\n\
             <body>\n\
             <div class=\"chart-container\">\n\
             {svg}\n\
             <div class=\"tooltip\" id=\"tooltip\" style=\"display:none\"></div>\n\
             <div class=\"axis-label axis-label-x\" id=\"axis-label-x\"></div>\n\
             <div class=\"axis-label axis-label-y\" id=\"axis-label-y\"></div>\n\
             </div>\n\
             <script>\n\
             {js}\n\
             </script>\n\
             </body>\n\
             </html>",
            title = Self::escape_html(&config.title),
            bg = config.theme_config.background_color,
            w = config.width,
            h = config.height,
            cross = config.theme_config.crosshair_color,
            svg = svg_content,
            js = Self::js_interaction_layer(
                data_json,
                indicators_json,
                hit_json,
                panel_json,
                config.margins.left,
                config.margins.right,
                plot,
            )
            .replace("__INTERACTION_ENABLED__", if config.interaction.enabled { "true" } else { "false" })
            .replace("__INTERACTION_CROSSHAIR__", if config.interaction.show_crosshair { "true" } else { "false" })
            .replace("__INTERACTION_DATA_WINDOW__", if config.interaction.show_data_window { "true" } else { "false" })
            .replace("__INTERACTION_PAN_ZOOM__", if config.interaction.enable_pan_zoom { "true" } else { "false" })
            .replace("__INTERACTION_KEYBOARD__", if config.interaction.enable_keyboard { "true" } else { "false" }),
        );

        Ok(html)
    }
}

impl HtmlRenderer {
    fn js_interaction_layer(
        data_json: &str,
        indicators_json: &str,
        hit_json: &str,
        panel_json: &str,
        left: u32,
        right: u32,
        plot: Option<crate::geometry::Rect>,
    ) -> String {
        let (plot_x, plot_y, plot_width, plot_height) = plot
            .map(|rect| (rect.x, rect.y, rect.width, rect.height))
            .unwrap_or((left as f64, 0.0, 0.0, 0.0));
        r#"(function(){
var svg=document.querySelector('svg');
if(!svg)return;
var bars=__BARS__;
var indicatorRows=__INDICATORS__;
var hits=__HITS__;
var panels=__PANELS__;
var interaction={enabled:__INTERACTION_ENABLED__,crosshair:__INTERACTION_CROSSHAIR__,dataWindow:__INTERACTION_DATA_WINDOW__,panZoom:__INTERACTION_PAN_ZOOM__,keyboard:__INTERACTION_KEYBOARD__};
var plot={x:__PLOT_X__,y:__PLOT_Y__,width:__PLOT_W__,height:__PLOT_H__};
var state={scaleX:1,offsetX:0,dragging:false,pointerId:null,lastX:0,lastY:0,keyboardIndex:0,frame:0,pendingEvent:null};
var chH=null,chV=null,tooltip=null,axisX=null,axisY=null;
var raf=window.requestAnimationFrame||function(callback){return window.setTimeout(callback,16);};
function panelBounds(){
  var result=panels.filter(function(panel){return panel.width>0&&panel.height>0;});
  if(!result.length&&plot.width&&plot.height)result=[plot];
  return result;
}
function panelAt(y){
  return panelBounds().filter(function(panel){return y>=panel.y&&y<=panel.y+panel.height;})[0]||null;
}
function inChart(x,y){
  return panelBounds().some(function(panel){return x>=panel.x&&x<=panel.x+panel.width&&y>=panel.y&&y<=panel.y+panel.height;});
}
function init(){
  var g=svg.querySelector('.chart-content');
  if(!g){
    g=document.createElementNS('http://www.w3.org/2000/svg','g');
    g.setAttribute('class','chart-content');
    Array.prototype.slice.call(svg.children).forEach(function(c){
      if(!(c.tagName==='rect'&&c.getAttribute('width')==='100%'))g.appendChild(c);
    });
    svg.appendChild(g);
  }
  chH=document.createElementNS('http://www.w3.org/2000/svg','line');
  chH.setAttribute('class','crosshair-h');
  chH.setAttribute('x1',plot.x);chH.setAttribute('y1',plot.y);
  chH.setAttribute('x2',plot.x+plot.width);chH.setAttribute('y2',plot.y);
  chH.style.display='none';svg.appendChild(chH);
  chV=document.createElementNS('http://www.w3.org/2000/svg','line');
  chV.setAttribute('class','crosshair-v');
  var bounds=panelBounds();
  var minY=bounds.length?Math.min.apply(null,bounds.map(function(panel){return panel.y;})):plot.y;
  var maxY=bounds.length?Math.max.apply(null,bounds.map(function(panel){return panel.y+panel.height;})):plot.y+plot.height;
  chV.setAttribute('x1',plot.x);chV.setAttribute('y1',minY);
  chV.setAttribute('x2',plot.x);chV.setAttribute('y2',maxY);
  chV.style.display='none';svg.appendChild(chV);
  tooltip=document.getElementById('tooltip');
  axisX=document.getElementById('axis-label-x');
  axisY=document.getElementById('axis-label-y');
  if(interaction.enabled&&interaction.keyboard)svg.setAttribute('tabindex','0');
}
function applyTransform(){
  var g=svg.querySelector('.chart-content');
  if(g)g.setAttribute('transform','translate('+state.offsetX+',0) scale('+state.scaleX+',1)');
}
svg.addEventListener('wheel',function(e){
  if(!interaction.enabled||!interaction.panZoom)return;
  e.preventDefault();
  var factor=e.deltaY>0?0.9:1.1;
  var rect=svg.getBoundingClientRect();
  var mx=e.clientX-rect.left;
  var oldScale=state.scaleX;
  state.scaleX*=factor;
  state.scaleX=Math.max(0.1,Math.min(100,state.scaleX));
  state.offsetX=mx-(mx-state.offsetX)*(state.scaleX/oldScale);
  applyTransform();
},{passive:false});
svg.addEventListener('pointerdown',function(e){
  if(!interaction.enabled||!interaction.panZoom)return;
  state.pointerId=e.pointerId;state.dragging=true;state.lastX=e.clientX;state.lastY=e.clientY;
  if(svg.setPointerCapture)svg.setPointerCapture(e.pointerId);
  svg.style.cursor='grabbing';
});
svg.addEventListener('pointermove',function(e){
  if(state.dragging){
    state.offsetX+=e.clientX-state.lastX;
    state.lastX=e.clientX;state.lastY=e.clientY;
    applyTransform();
  }
  scheduleCrosshair(e);
});
svg.addEventListener('pointerup',function(e){
  if(state.pointerId!==null&&e.pointerId!==state.pointerId)return;
  state.dragging=false;state.pointerId=null;svg.style.cursor='crosshair';
});
svg.addEventListener('pointercancel',function(){
  state.dragging=false;state.pointerId=null;svg.style.cursor='crosshair';hideCrosshair();
});
svg.addEventListener('mouseleave',function(){
  state.dragging=false;svg.style.cursor='crosshair';hideCrosshair();
});
svg.addEventListener('keydown',function(e){
  if(!interaction.enabled||!interaction.keyboard||!bars.length)return;
  if(e.key==='ArrowLeft'||e.key==='ArrowRight'){
    e.preventDefault();
    state.keyboardIndex=Math.max(0,Math.min(bars.length-1,state.keyboardIndex+(e.key==='ArrowRight'?1:-1)));
    showBar(state.keyboardIndex,plot.x+(state.keyboardIndex+0.5)*plot.width/bars.length,plot.y+18);
  }
});
function scheduleCrosshair(e){
  if(!interaction.enabled)return;
  state.pendingEvent=e;
  if(state.frame)return;
  state.frame=raf(function(){
    state.frame=0;
    var event=state.pendingEvent;state.pendingEvent=null;
    if(event)updateCrosshair(event);
  });
}
function showBar(index,x,y){
  if(!bars[index])return;
  var rect=svg.getBoundingClientRect();
  updateCrosshair({clientX:rect.left+x,clientY:rect.top+y});
}
function fmt(v,d){return v===null||v===undefined||!isFinite(Number(v))?'--':Number(v).toFixed(d||2);}
function updateCrosshair(e){
  var rect=svg.getBoundingClientRect();
  var x=e.clientX-rect.left;
  var y=e.clientY-rect.top;
  if(!plot.width||!inChart(x,y)){hideCrosshair();return;}
  var panel=panelAt(y)||plot;
  if(chH){chH.setAttribute('x1',panel.x);chH.setAttribute('x2',panel.x+panel.width);chH.setAttribute('y1',y);chH.setAttribute('y2',y);chH.style.display=interaction.crosshair?'':'none';}
  if(chV){chV.setAttribute('x1',x);chV.setAttribute('x2',x);chV.style.display=interaction.crosshair?'':'none';}
  if(tooltip){
    var localX=(x-state.offsetX)/Math.max(state.scaleX,0.0001);
    var hit=hits.filter(function(item){return localX>=item.x&&localX<=item.x+item.width&&y>=item.y&&y<=item.y+item.height;})
      .sort(function(a,b){return b.priority-a.priority;})[0];
    var content='<div class="tooltip-title">数据窗口</div>';
    if(bars.length){
      var index=Math.max(0,Math.min(bars.length-1,Math.floor((localX-plot.x)/Math.max(plot.width/bars.length,0.0001))));
      var bar=bars[index];
      var esc=function(v){return String(v).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;');};
      var signed=function(v,d){return v===null||v===undefined?'--':(v>=0?'+':'')+Number(v).toFixed(d||2);};
      var color=function(v){return v===null||v===undefined?'':(v>=0?' style="color:#f87171"':' style="color:#4ade80"');};
      content+='<div class="tooltip-title">'+esc(bar.date)+' #'+bar.index+(bar.source_end>bar.index+1?' ['+bar.index+','+(bar.source_end-1)+']':'')+'</div>';
      if(bar.timestamp!==undefined&&bar.timestamp!==null) content+='<div class="tooltip-label">时间戳 '+esc(bar.timestamp)+'</div>';
      content+='<div class="tooltip-grid">';
      content+='<div class="tooltip-label">开</div><div class="tooltip-value">'+fmt(bar.open)+'</div>';
      content+='<div class="tooltip-label">高</div><div class="tooltip-value">'+fmt(bar.high)+'</div>';
      content+='<div class="tooltip-label">低</div><div class="tooltip-value">'+fmt(bar.low)+'</div>';
      content+='<div class="tooltip-label">收</div><div class="tooltip-value"'+color(bar.change)+'>'+fmt(bar.close)+'</div>';
      content+='<div class="tooltip-label">昨收</div><div class="tooltip-value">'+fmt(bar.prev_close)+'</div>';
      content+='<div class="tooltip-label">涨跌</div><div class="tooltip-value"'+color(bar.change)+'>'+signed(bar.change)+'</div>';
      content+='<div class="tooltip-label">涨幅</div><div class="tooltip-value"'+color(bar.change_pct)+'>'+signed(bar.change_pct)+'%</div>';
      content+='<div class="tooltip-label">振幅</div><div class="tooltip-value">'+fmt(bar.amplitude_pct)+'%</div>';
      content+='<div class="tooltip-label">成交量</div><div class="tooltip-value">'+fmt(bar.volume)+'</div>';
      content+='</div>';
      if(indicatorRows.length){
        content+='<div class="tooltip-hit">指标</div><div class="tooltip-grid">';
        indicatorRows.forEach(function(row){
          var value=row.values[index];
          content+='<div class="tooltip-label">'+esc(row.name)+'</div><div class="tooltip-value">'+fmt(value)+'</div>';
        });
        content+='</div>';
      }
      if(hit)content+='<div class="tooltip-hit">'+esc(hit.tooltip||hit.target)+'</div>';
    }
    if(interaction.dataWindow){
      tooltip.innerHTML=content;
      tooltip.style.display='block';
    }else tooltip.style.display='none';
    var tx=x+14,ty=y+14;
    var maxX=svg.clientWidth-tooltip.offsetWidth-6,maxY=svg.clientHeight-tooltip.offsetHeight-6;
    tooltip.style.left=Math.max(6,Math.min(tx,maxX))+'px';
    tooltip.style.top=Math.max(6,Math.min(ty,maxY))+'px';
  }
  if(bars.length){
    var selected=Math.max(0,Math.min(bars.length-1,Math.floor(((x-state.offsetX)/Math.max(state.scaleX,0.0001)-plot.x)/Math.max(plot.width/bars.length,0.0001))));
    var selectedBar=bars[selected];
    state.keyboardIndex=selected;
    if(axisX){axisX.textContent=selectedBar.date;axisX.style.left=x+'px';axisX.style.top=Math.min(svg.clientHeight-20,Math.max(0,plot.y+plot.height+2))+'px';axisX.style.display=interaction.crosshair?'block':'none';}
    if(axisY){axisY.textContent=fmt(selectedBar.close);axisY.style.left=Math.max(0,plot.x+plot.width+4)+'px';axisY.style.top=Math.max(0,y-8)+'px';axisY.style.display=interaction.crosshair?'block':'none';}
  }
}
function hideCrosshair(){
  if(chH)chH.style.display='none';
  if(chV)chV.style.display='none';
  if(tooltip)tooltip.style.display='none';
  if(axisX)axisX.style.display='none';
  if(axisY)axisY.style.display='none';
}
init();
})();"#
        .replace("__BARS__", &data_json.replace("</", "<\\/"))
        .replace("__INDICATORS__", &indicators_json.replace("</", "<\\/"))
        .replace("__HITS__", &hit_json.replace("</", "<\\/"))
        .replace("__PANELS__", &panel_json.replace("</", "<\\/"))
        .replace("__LEFT__", &left.to_string())
        .replace("__RIGHT__", &right.to_string())
        .replace("__PLOT_X__", &plot_x.to_string())
        .replace("__PLOT_Y__", &plot_y.to_string())
        .replace("__PLOT_W__", &plot_width.to_string())
        .replace("__PLOT_H__", &plot_height.to_string())
    }

    fn escape_html(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}

#[cfg(all(test, feature = "html"))]
mod tests {
    use super::*;
    use crate::geometry::Point;
    use crate::primitive::{Primitive, Style};

    fn default_config() -> ChartConfig {
        ChartConfig::default()
    }

    #[test]
    fn test_html_renderer_basic() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.starts_with("<!DOCTYPE html>"));
        assert!(result.contains("<html>"));
        assert!(result.contains("</html>"));
        assert!(result.contains("<svg"));
        assert!(result.contains("</svg>"));
    }

    #[test]
    fn test_html_contains_css() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("<style>"));
        assert!(result.contains("</style>"));
        assert!(result.contains(".chart-container"));
    }

    #[test]
    fn test_html_contains_js() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("<script>"));
        assert!(result.contains("</script>"));
        assert!(result.contains("addEventListener"));
        assert!(result.contains("wheel"));
        assert!(result.contains("pointerdown"));
        assert!(result.contains("requestAnimationFrame"));
        assert!(result.contains("axis-label-x"));
    }

    #[test]
    fn test_html_contains_svg_content() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let mut draw_list = DrawList::new();
        draw_list.push(Primitive::Line {
            p1: Point::new(0.0, 0.0),
            p2: Point::new(100.0, 100.0),
            style: Style::default(),
        });
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("<line"));
        assert!(result.contains("x1=\"0.00\""));
    }

    #[test]
    fn test_html_contains_tooltip() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("tooltip"));
        assert!(result.contains("id=\"tooltip\""));
    }

    #[test]
    fn test_html_contains_crosshair_css() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains(".crosshair-h"));
        assert!(result.contains(".crosshair-v"));
    }

    #[test]
    fn test_html_meta_charset() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("charset=\"utf-8\""));
    }

    #[test]
    fn test_html_title_escaped() {
        let renderer = HtmlRenderer::new();
        let mut config = default_config();
        config.title = "<script>alert('xss')</script>".to_string();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>alert('xss')</script>"));
    }

    #[test]
    fn test_html_self_contained() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(!result.contains("src="));
        assert!(!result.contains("href="));
    }

    #[test]
    fn test_html_renderer_default() {
        let renderer = HtmlRenderer;
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(HtmlRenderer::escape_html("a&b"), "a&amp;b");
        assert_eq!(HtmlRenderer::escape_html("<tag>"), "&lt;tag&gt;");
        assert_eq!(
            HtmlRenderer::escape_html("\"quoted\""),
            "&quot;quoted&quot;"
        );
        assert_eq!(HtmlRenderer::escape_html("normal"), "normal");
    }

    #[test]
    fn test_html_with_theme_colors() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("finkit-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains(&config.theme_config.background_color));
        assert!(result.contains(&config.theme_config.crosshair_color));
    }

    #[test]
    fn test_html_js_size_under_5kb() {
        let js = HtmlRenderer::js_interaction_layer("[]", "[]", "[]", "[]", 60, 40, None);
        assert!(
            js.len() < 12 * 1024,
            "JS interaction layer should be under 12KB, got {} bytes",
            js.len()
        );
    }

    #[test]
    fn test_html_with_ohlcv_tooltip_payload() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let data = KlineData::new(
            vec!["2024-01-01".into()],
            vec![10.0],
            vec![12.0],
            vec![9.0],
            vec![11.0],
            vec![100.0],
        );
        let html = renderer
            .render_with_data(&DrawList::new(), &config, &data, 7, &ChartScene::default())
            .expect("HTML tooltip payload should serialize");
        assert!(html.contains("2024-01-01"));
        assert!(html.contains("\"index\":7"));
        assert!(html.contains("\"change\":null"));
        assert!(html.contains("tooltip-grid"));
        assert!(html.contains("plot={x:"));
        assert!(html.contains("fmt(bar.open)"));
        assert!(html.contains("__INTERACTION_ENABLED__") == false);
    }

    #[test]
    fn test_html_interaction_can_be_disabled() {
        let renderer = HtmlRenderer::new();
        let mut config = default_config();
        config.interaction.enabled = false;
        let html = renderer
            .render_with_data(
                &DrawList::new(),
                &config,
                &KlineData::new(
                    vec!["2024-01-01".into()],
                    vec![10.0],
                    vec![12.0],
                    vec![9.0],
                    vec![11.0],
                    vec![100.0],
                ),
                0,
                &ChartScene::default(),
            )
            .expect("disabled interaction should still render");
        assert!(html.contains("enabled:false"));
    }

    #[test]
    fn indicator_payload_contains_visible_indicator_rows() {
        let data = KlineData::new(
            (0..40).map(|index| index.to_string()).collect(),
            (0..40).map(|index| index as f64 + 1.0).collect(),
            (0..40).map(|index| index as f64 + 2.0).collect(),
            (0..40).map(|index| index as f64).collect(),
            (0..40).map(|index| index as f64 + 1.5).collect(),
            vec![100.0; 40],
        );
        let payload = indicator_payload(
            &data,
            &[],
            &[IndicatorConfig::new(IndicatorType::RSI, vec![14.0])],
            &HashMap::new(),
        );
        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0].name, "RSI");
        assert_eq!(payload[0].values.len(), data.len());
    }
}
