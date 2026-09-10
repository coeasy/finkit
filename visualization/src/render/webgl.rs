//! WebGL2 renderer for dense OHLCV charts.
//!
//! The renderer keeps the semantic scene and tooltip contract of the SVG and
//! Canvas backends, but moves the repeated price/volume geometry into two
//! instanced WebGL draw calls.  The generated document is self contained and
//! falls back to a 2D canvas when WebGL2 is unavailable.

use crate::config::{ChartConfig, IndicatorConfig};
use crate::data::KlineData;
use crate::error::{Result, VisualizationError};
use crate::geometry::Rect;
use crate::primitive::DrawList;
use crate::scene::{ChartScene, PanelId};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::Serialize;
use std::collections::HashMap;

use super::html::indicator_payload;
use super::{CanvasRenderer, Renderer};

#[derive(Debug, Clone, Serialize)]
struct GpuBar {
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
}

#[derive(Debug, Clone, Serialize)]
struct GpuHitRegion {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    priority: i32,
    target: String,
    tooltip: Option<String>,
}

/// Instanced WebGL2 backend for large price/volume datasets.
pub struct WebGlRenderer;

impl WebGlRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Render a dense chart with GPU price/volume geometry and a Canvas 2D
    /// overlay for grid, indicators, Chan structures and labels.
    pub fn render_with_data(
        &self,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        main_plot: Rect,
        volume_plot: Option<Rect>,
        scene: &ChartScene,
        overlay_draw_list: &DrawList,
    ) -> Result<String> {
        let empty = DrawList::new();
        self.render_with_layers(
            config,
            data,
            source_offset,
            source_ranges,
            main_plot,
            volume_plot,
            scene,
            overlay_draw_list,
            &empty,
            &empty,
        )
    }

    /// Render with separate static background and x-transformable semantic
    /// layers. This keeps grid/axes stable while allowing browser zoom and
    /// pan to move Chan, event and indicator primitives together with bars.
    pub fn render_with_layers(
        &self,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        main_plot: Rect,
        volume_plot: Option<Rect>,
        scene: &ChartScene,
        background_draw_list: &DrawList,
        semantic_draw_list: &DrawList,
        indicator_draw_list: &DrawList,
    ) -> Result<String> {
        self.render_with_layers_and_indicators(
            config,
            data,
            source_offset,
            source_ranges,
            main_plot,
            volume_plot,
            scene,
            background_draw_list,
            semantic_draw_list,
            indicator_draw_list,
            &[],
            &HashMap::new(),
        )
    }

    /// Render layers and serialize the visible indicator values and semantic
    /// hit regions used by the TDX-style floating data window.
    pub fn render_with_layers_and_indicators(
        &self,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        main_plot: Rect,
        volume_plot: Option<Rect>,
        scene: &ChartScene,
        background_draw_list: &DrawList,
        semantic_draw_list: &DrawList,
        indicator_draw_list: &DrawList,
        indicator_configs: &[IndicatorConfig],
        custom_series: &HashMap<String, Vec<f64>>,
    ) -> Result<String> {
        self.render_with_layers_and_indicators_prefer_webgpu(
            config,
            data,
            source_offset,
            source_ranges,
            main_plot,
            volume_plot,
            scene,
            background_draw_list,
            semantic_draw_list,
            indicator_draw_list,
            indicator_configs,
            custom_series,
            false,
        )
    }

    /// Render with an optional WebGPU preference. WebGPU is opt-in; the
    /// default WebGL renderer starts with WebGL2 and falls back to Canvas 2D.
    pub fn render_with_layers_and_indicators_prefer_webgpu(
        &self,
        config: &ChartConfig,
        data: &KlineData,
        source_offset: usize,
        source_ranges: &[(usize, usize)],
        main_plot: Rect,
        volume_plot: Option<Rect>,
        scene: &ChartScene,
        background_draw_list: &DrawList,
        semantic_draw_list: &DrawList,
        indicator_draw_list: &DrawList,
        indicator_configs: &[IndicatorConfig],
        custom_series: &HashMap<String, Vec<f64>>,
        prefer_webgpu: bool,
    ) -> Result<String> {
        if data.is_empty() || !data.validate() || !data.validate_timestamps() {
            return Err(VisualizationError::ConversionError {
                message: "WebGL renderer received invalid K-line data".to_string(),
            });
        }
        let bars: Vec<GpuBar> = (0..data.len())
            .map(|index| {
                let (source_start, source_end) = source_ranges
                    .get(index)
                    .copied()
                    .unwrap_or((source_offset + index, source_offset + index + 1));
                GpuBar {
                    index: source_start,
                    source_end,
                    date: data.dates[index].clone(),
                    timestamp: data.timestamps.get(index).copied(),
                    open: data.opens[index],
                    high: data.highs[index],
                    low: data.lows[index],
                    close: data.closes[index],
                    volume: data.volumes[index],
                }
            })
            .collect();
        let bars_json = serde_json::to_string(&bars)
            .map_err(|error| VisualizationError::SerializationError {
                message: format!("Failed to serialize WebGL data window: {error}"),
            })?
            .replace("</", "<\\/");
        // GPU instance layout: OHLCV + local source start/end.  The final two
        // lanes let the browser switch to an envelope level without losing
        // the source-coordinate mapping used by the overlay and tooltip.
        let mut values = Vec::with_capacity(data.len() * 7);
        for index in 0..data.len() {
            values.extend([
                data.opens[index] as f32,
                data.highs[index] as f32,
                data.lows[index] as f32,
                data.closes[index] as f32,
                data.volumes[index] as f32,
                index as f32,
                (index + 1) as f32,
            ]);
        }
        let mut values_bytes = Vec::with_capacity(values.len() * std::mem::size_of::<f32>());
        for value in &values {
            values_bytes.extend_from_slice(&value.to_le_bytes());
        }
        let values_b64 = BASE64.encode(values_bytes);
        let background_commands_json =
            CanvasRenderer::command_json(background_draw_list)?.replace("</", "<\\/");
        let semantic_commands_json =
            CanvasRenderer::command_json(semantic_draw_list)?.replace("</", "<\\/");
        let indicator_commands_json =
            CanvasRenderer::command_json(indicator_draw_list)?.replace("</", "<\\/");
        let main_panel = [main_plot.x, main_plot.y, main_plot.width, main_plot.height];
        let volume_panel = volume_plot
            .or_else(|| {
                scene
                    .panels
                    .iter()
                    .find(|panel| panel.id == PanelId::Volume)
                    .map(|panel| panel.rect)
            })
            .map(|rect| [rect.x, rect.y, rect.width, rect.height]);
        let main_panel_json =
            serde_json::to_string(&main_panel).unwrap_or_else(|_| "[0,0,0,0]".to_string());
        let volume_panel_json =
            serde_json::to_string(&volume_panel).unwrap_or_else(|_| "null".to_string());
        let price_min = data.lows.iter().copied().fold(f64::INFINITY, f64::min);
        let price_max = data.highs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let volume_max = data.volumes.iter().copied().fold(0.0_f64, f64::max);
        // Dynamic browser-side LOD works in the current display-coordinate
        // space.  A row may already represent a source range; the browser
        // envelope then combines display rows while `GpuBar` keeps the source
        // range for tooltip/audit purposes.  This keeps semantic overlays in
        // the same coordinate space instead of trying to mix raw and reduced
        // indices.
        let dynamic_lod = source_ranges.len() == data.len()
            && source_ranges.iter().all(|&(start, end)| end > start);
        let background = CanvasRenderer::escape_html(&config.theme_config.background_color);
        let title = CanvasRenderer::escape_html(&config.title);
        let up_color = config.color_scheme.up_color();
        let down_color = config.color_scheme.down_color();
        let overlay_json = format!(
            "{{\"background\":{background_commands_json},\"semantic\":{semantic_commands_json},\"indicator\":{indicator_commands_json}}}"
        );
        let indicators_json = serde_json::to_string(&indicator_payload(
            data,
            source_ranges,
            indicator_configs,
            custom_series,
        ))
        .map_err(|error| VisualizationError::SerializationError {
            message: format!("Failed to serialize WebGL indicator data: {error}"),
        })?
        .replace("</", "<\\/");
        let hit_regions_json = serde_json::to_string(
            &scene
                .hit_regions
                .iter()
                .map(|region| GpuHitRegion {
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
        .map_err(|error| VisualizationError::SerializationError {
            message: format!("Failed to serialize WebGL hit regions: {error}"),
        })?
        .replace("</", "<\\/");

        let mut html = r#"<!DOCTYPE html>
<html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>__TITLE__</title>
<style>*{box-sizing:border-box}body{margin:0;background:__BACKGROUND__;display:flex;justify-content:center;align-items:center;min-height:100vh}.finkit-webgl-shell{position:relative;width:__WIDTH__px;height:__HEIGHT__px;touch-action:none}.finkit-webgl-shell canvas{position:absolute;inset:0;width:100%;height:100%;display:block}.finkit-webgl-gpu{z-index:1}.finkit-webgl-overlay{z-index:2;pointer-events:none}.finkit-webgl-tooltip{position:fixed;z-index:3;display:none;min-width:220px;pointer-events:none;padding:8px 10px;border:1px solid rgba(148,163,184,.45);border-radius:3px;background:rgba(14,18,28,.94);color:#f3f4f6;font:12px/1.45 sans-serif;box-shadow:0 3px 12px rgba(0,0,0,.22)}.finkit-webgl-tooltip b{display:block;color:#fff;border-bottom:1px solid rgba(148,163,184,.3);padding-bottom:4px;margin-bottom:4px}.finkit-webgl-tooltip .gpu-tooltip-section{margin-top:5px;padding-top:4px;border-top:1px solid rgba(148,163,184,.3);color:#fbbf24}.finkit-webgl-fallback{position:absolute;left:8px;top:8px;z-index:4;display:none;color:#94a3b8;font:12px sans-serif}</style></head>
 <body><div class="finkit-webgl-shell" data-renderer="webgl2" data-packed-format="f32-base64" aria-label="__TITLE__">
<canvas class="finkit-webgl-gpu" width="__WIDTH__" height="__HEIGHT__"></canvas><canvas class="finkit-webgl-overlay" width="__WIDTH__" height="__HEIGHT__"></canvas>
<div class="finkit-webgl-tooltip" role="tooltip"></div><div class="finkit-webgl-fallback">WebGL2 unavailable; using Canvas 2D</div></div>
<script>(async function(){'use strict';
var shell=document.querySelector('.finkit-webgl-shell');var gpu=shell&&shell.querySelector('.finkit-webgl-gpu');var overlay=shell&&shell.querySelector('.finkit-webgl-overlay');var tip=shell&&shell.querySelector('.finkit-webgl-tooltip');var fallback=shell&&shell.querySelector('.finkit-webgl-fallback');if(!shell||!gpu||!overlay)return;
 function decodeF32(encoded){var raw=atob(encoded),bytes=new Uint8Array(raw.length);for(var i=0;i<raw.length;i++)bytes[i]=raw.charCodeAt(i);return new Float32Array(bytes.buffer)}
 var bars=__BARS__;var count=bars.length;var initialPacked=decodeF32('__VALUES_B64__');var capacity=1;while(capacity<Math.max(1024,count*2))capacity*=2;var packed=new Float32Array(capacity*7);packed.set(initialPacked);var ringEnabled=false;var ringHead=0;var ringCapacity=0;var nextSourceIndex=bars.length?Number(bars[bars.length-1].source_end||bars.length):0;var mainPanel=__MAIN_PANEL__;var volumePanel=__VOLUME_PANEL__;var priceMin=__PRICE_MIN__;var priceMax=__PRICE_MAX__;var volumeMax=Math.max(__VOLUME_MAX__,1);var dynamicLod=__DYNAMIC_LOD__;var overlayData=__OVERLAY__;var backgroundCommands=overlayData.background;var semanticCommands=overlayData.semantic;var indicatorCommands=overlayData.indicator;var indicatorRows=__INDICATORS__;var hitRegions=__HITS__;var interaction={enabled:__INTERACTION_ENABLED__,crosshair:__INTERACTION_CROSSHAIR__,dataWindow:__INTERACTION_DATA_WINDOW__,panZoom:__INTERACTION_PAN_ZOOM__};var hover=null;var viewStart=0;var viewEnd=count;var dragging=false;var dragX=0;var activePacked=packed;var activeCount=count;var activeBars=bars;var activeBucket=1;var activeSourceStart=0;var levelCache={};var gpuRawUpdate=null;var gpuResize=null;
function esc(v){return String(v).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;').replace(/'/g,'&#39;')}function color(hex){var h=String(hex||'#888').replace('#','');if(h.length===8)h=h.slice(0,6);return [parseInt(h.slice(0,2),16)/255,parseInt(h.slice(2,4),16)/255,parseInt(h.slice(4,6),16)/255,1]}
var upColor=color('__UP_COLOR__'),downColor=color('__DOWN_COLOR__'),background='__BACKGROUND__';
 function physicalIndex(index){return ringEnabled?((ringHead+index)%ringCapacity):index}function rawValue(index,lane){return packed[physicalIndex(index)*7+lane]}function makeEnvelope(bucket,start,end){bucket=Math.max(1,Math.floor(bucket));start=Math.max(0,Math.floor(start));end=Math.min(count,Math.ceil(end));if(end<=start)end=Math.min(count,start+1);var alignedStart=Math.floor(start/bucket)*bucket,key=bucket+':'+alignedStart+':'+end+':'+ringHead;if(levelCache[key])return levelCache[key];var n=Math.ceil((end-alignedStart)/bucket),out=new Float32Array(n*7),meta=new Array(n);for(var b=0;b<n;b++){var s=alignedStart+b*bucket,e=Math.min(count,s+bucket),base=b*7,high=-Infinity,low=Infinity,volume=0;for(var i=s;i<e;i++){high=Math.max(high,rawValue(i,1));low=Math.min(low,rawValue(i,2));volume+=Number.isFinite(rawValue(i,4))?rawValue(i,4):0}out[base]=rawValue(s,0);out[base+1]=high;out[base+2]=low;out[base+3]=rawValue(e-1,3);out[base+4]=volume;out[base+5]=s;out[base+6]=e;var firstBar=bars[s],lastBar=bars[e-1];meta[b]={index:firstBar.index,source_end:lastBar.source_end,date:firstBar.date,timestamp:firstBar.timestamp,open:out[base],high:high,low:low,close:out[base+3],volume:volume};}var level={packed:out,bars:meta,count:n,start:alignedStart};levelCache[key]=level;var keys=Object.keys(levelCache);if(keys.length>24)delete levelCache[keys[0]];return level}
 function selectActive(){if(!dynamicLod&&!ringEnabled){activePacked=packed;activeBars=bars;activeCount=count;activeBucket=1;activeSourceStart=0;shell.dataset.lodBucket='1';shell.dataset.activeCount=String(activeCount);return}var span=Math.max(1,viewEnd-viewStart),pixels=Math.max(1,mainPanel[2]*(window.devicePixelRatio||1)),bucket=dynamicLod&&span>pixels*1.5?Math.max(2,Math.ceil(span/(pixels*1.5))):1;if(bucket>1){var power=1;while(power<bucket)power*=2;bucket=power}var margin=bucket>1?Math.max(2,bucket*2):0,start=Math.max(0,Math.floor(viewStart)-margin),end=Math.min(count,Math.ceil(viewEnd)+margin),level=makeEnvelope(bucket,start,end);activePacked=level.packed;activeBars=level.bars;activeCount=level.count;activeBucket=bucket;activeSourceStart=level.start;shell.dataset.lodBucket=String(activeBucket);shell.dataset.activeCount=String(activeCount)}
 function recalcBounds(){priceMin=Infinity;priceMax=-Infinity;volumeMax=1;for(var i=0;i<count;i++){priceMin=Math.min(priceMin,rawValue(i,2));priceMax=Math.max(priceMax,rawValue(i,1));var volume=rawValue(i,4);volumeMax=Math.max(volumeMax,Number.isFinite(volume)?volume:0)}}
function drawCommands(ctx,list){function style(s){ctx.globalAlpha=s.opacity;ctx.strokeStyle=s.stroke||'transparent';ctx.fillStyle=s.fill||'transparent';ctx.lineWidth=s.lineWidth;ctx.setLineDash(s.lineDash||[]);ctx.font=s.fontSize+'px '+s.fontFamily}function path(p,close){if(!p.length)return;ctx.beginPath();ctx.moveTo(p[0][0],p[0][1]);for(var i=1;i<p.length;i++)ctx.lineTo(p[i][0],p[i][1]);if(close)ctx.closePath()}list.forEach(function(c){var r;switch(c.type){case'line':style(c.style);ctx.beginPath();ctx.moveTo(c.p1[0],c.p1[1]);ctx.lineTo(c.p2[0],c.p2[1]);if(c.style.stroke)ctx.stroke();break;case'rect':style(c.style);r=c.rect;if(c.style.fill)ctx.fillRect(r.x,r.y,r.width,r.height);if(c.style.stroke)ctx.strokeRect(r.x,r.y,r.width,r.height);break;case'filledRect':r=c.rect;ctx.globalAlpha=1;ctx.fillStyle=c.fill;ctx.fillRect(r.x,r.y,r.width,r.height);if(c.stroke){ctx.strokeStyle=c.stroke;ctx.strokeRect(r.x,r.y,r.width,r.height)}break;case'polygon':style(c.style);path(c.points,true);if(c.style.fill)ctx.fill();if(c.style.stroke)ctx.stroke();break;case'path':style(c.style);path(c.points,c.close);if(c.close&&c.style.fill)ctx.fill();if(c.style.stroke)ctx.stroke();break;case'circle':style(c.style);ctx.beginPath();ctx.arc(c.center[0],c.center[1],c.radius,0,Math.PI*2);if(c.style.fill)ctx.fill();if(c.style.stroke)ctx.stroke();break;case'text':style(c.style);ctx.fillStyle=c.style.fill||c.style.stroke||'#000';ctx.fillText(c.content,c.position[0],c.position[1]);break;case'group':ctx.save();if(c.transform)ctx.transform(c.transform[0],c.transform[3],c.transform[1],c.transform[4],c.transform[2],c.transform[5]);drawCommands(ctx,c.primitives);ctx.restore();break}})}
 function visibleSpan(){return Math.max(1,viewEnd-viewStart)}function clampView(){var span=Math.max(1,Math.min(count,viewEnd-viewStart));viewStart=Math.max(0,Math.min(Math.max(0,count-span),viewStart));viewEnd=viewStart+span}function redraw(){clampView();if(webgpuDraw){webgpuDraw()}else if(ready){drawGpu()}else{drawCpu()}drawOverlay()}function drawOverlay(){var ctx=overlay.getContext('2d');var dpr=Math.max(1,window.devicePixelRatio||1);overlay.width=__WIDTH__*dpr;overlay.height=__HEIGHT__*dpr;ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,__WIDTH__,__HEIGHT__);drawCommands(ctx,backgroundCommands);var span=visibleSpan(),scale=count/span;ctx.save();ctx.beginPath();ctx.rect(mainPanel[0],0,mainPanel[2],__HEIGHT__);ctx.clip();ctx.translate(mainPanel[0]-viewStart/span*mainPanel[2],0);ctx.scale(scale,1);ctx.translate(-mainPanel[0],0);drawCommands(ctx,semanticCommands);drawCommands(ctx,indicatorCommands);ctx.restore();if(hover&&interaction.crosshair){ctx.save();ctx.strokeStyle='rgba(148,163,184,.55)';ctx.lineWidth=1;ctx.setLineDash([4,4]);ctx.beginPath();ctx.moveTo(hover.x,mainPanel[1]);ctx.lineTo(hover.x,mainPanel[1]+mainPanel[3]);if(volumePanel){ctx.moveTo(hover.x,volumePanel[1]);ctx.lineTo(hover.x,volumePanel[1]+volumePanel[3])}ctx.moveTo(mainPanel[0],hover.y);ctx.lineTo(mainPanel[0]+mainPanel[2],hover.y);ctx.stroke();ctx.restore()}}
var webgpuDraw=null;var gpuUpload=null;var gl=gpu.getContext('webgl2',{antialias:false,preserveDrawingBuffer:false});var ready=false;var bodyProgram,wickProgram,buffer,bodyVertices;
function shader(type,source){var s=gl.createShader(type);gl.shaderSource(s,source);gl.compileShader(s);if(!gl.getShaderParameter(s,gl.COMPILE_STATUS))throw new Error(gl.getShaderInfoLog(s));return s}function program(v,f){var p=gl.createProgram();gl.attachShader(p,shader(gl.VERTEX_SHADER,v));gl.attachShader(p,shader(gl.FRAGMENT_SHADER,f));gl.linkProgram(p);if(!gl.getProgramParameter(p,gl.LINK_STATUS))throw new Error(gl.getProgramInfoLog(p));return p}
var vertexBody='#version 300 es\nin vec2 aVertex;in float aOpen;in float aHigh;in float aLow;in float aClose;in float aVolume;in float aSourceStart;in float aSourceEnd;uniform vec2 uSize;uniform vec4 uPanel;uniform float uMin;uniform float uMax;uniform float uCount;uniform float uStart;uniform float uEnd;uniform float uBarWidth;uniform bool uVolumeMode;out float vUp;void main(){float top=uVolumeMode?aVolume:max(aOpen,aClose);float bottom=uVolumeMode?0.0:min(aOpen,aClose);float value=mix(bottom,top,(aVertex.y+1.0)*0.5);float frac=(value-uMin)/max(uMax-uMin,0.000001);float span=max(uEnd-uStart,1.0);float center=(aSourceStart+aSourceEnd)*0.5;float width=max(aSourceEnd-aSourceStart,1.0)/span*uBarWidth;float x=(center-uStart)/span+aVertex.x*width;float px=uPanel.x+x*uPanel.z;float py=uPanel.y+(1.0-frac)*uPanel.w;gl_Position=vec4(px/uSize.x*2.0-1.0,1.0-py/uSize.y*2.0,0,1);vUp=aClose>=aOpen?1.0:0.0;}';
var vertexWick='#version 300 es\nin float aOpen;in float aHigh;in float aLow;in float aClose;in float aVolume;in float aSourceStart;in float aSourceEnd;uniform vec2 uSize;uniform vec4 uPanel;uniform float uMin;uniform float uMax;uniform float uCount;uniform float uStart;uniform float uEnd;out float vUp;void main(){float value=gl_VertexID==0?aLow:aHigh;float frac=(value-uMin)/max(uMax-uMin,0.000001);float span=max(uEnd-uStart,1.0);float center=(aSourceStart+aSourceEnd)*0.5;float x=(center-uStart)/span;float px=uPanel.x+x*uPanel.z;float py=uPanel.y+(1.0-frac)*uPanel.w;gl_Position=vec4(px/uSize.x*2.0-1.0,1.0-py/uSize.y*2.0,0,1);vUp=aClose>=aOpen?1.0:0.0;}';
var fragment='#version 300 es\nprecision mediump float;in float vUp;uniform vec4 uUpColor;uniform vec4 uDownColor;out vec4 outColor;void main(){outColor=vUp>0.5?uUpColor:uDownColor;}';
function attrib(p,name,size,offset){var loc=gl.getAttribLocation(p,name);if(loc<0)return;gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,size,gl.FLOAT,false,28,offset);gl.vertexAttribDivisor(loc,1)}function uniforms(p,panel,min,max,n,mode,start,end){gl.uniform2f(gl.getUniformLocation(p,'uSize'),__WIDTH__,__HEIGHT__);gl.uniform4f(gl.getUniformLocation(p,'uPanel'),panel[0],panel[1],panel[2],panel[3]);gl.uniform1f(gl.getUniformLocation(p,'uMin'),min);gl.uniform1f(gl.getUniformLocation(p,'uMax'),max);gl.uniform1f(gl.getUniformLocation(p,'uCount'),n);gl.uniform1f(gl.getUniformLocation(p,'uStart'),start);gl.uniform1f(gl.getUniformLocation(p,'uEnd'),end);gl.uniform1f(gl.getUniformLocation(p,'uBarWidth'),.82);var modeLoc=gl.getUniformLocation(p,'uVolumeMode');if(modeLoc)gl.uniform1i(modeLoc,mode?1:0);gl.uniform4fv(gl.getUniformLocation(p,'uUpColor'),upColor);gl.uniform4fv(gl.getUniformLocation(p,'uDownColor'),downColor)}
function drawGpu(){selectActive();var dpr=Math.max(1,window.devicePixelRatio||1);gpu.width=__WIDTH__*dpr;gpu.height=__HEIGHT__*dpr;gl.viewport(0,0,gpu.width, gpu.height);gl.clearColor.apply(gl,[].concat(color(background)));gl.clear(gl.COLOR_BUFFER_BIT);if(gpuUpload)gpuUpload(activePacked);var span=visibleSpan();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.useProgram(bodyProgram);attrib(bodyProgram,'aOpen',1,0);attrib(bodyProgram,'aHigh',1,4);attrib(bodyProgram,'aLow',1,8);attrib(bodyProgram,'aClose',1,12);attrib(bodyProgram,'aVolume',1,16);attrib(bodyProgram,'aSourceStart',1,20);attrib(bodyProgram,'aSourceEnd',1,24);var p=gl.getAttribLocation(bodyProgram,'aVertex');gl.bindBuffer(gl.ARRAY_BUFFER,bodyVertices);gl.enableVertexAttribArray(p);gl.vertexAttribPointer(p,2,gl.FLOAT,false,0,0);gl.vertexAttribDivisor(p,0);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);uniforms(bodyProgram,mainPanel,priceMin,priceMax,activeCount,false,viewStart,viewEnd);gl.drawArraysInstanced(gl.TRIANGLES,0,6,activeCount);if(volumePanel){uniforms(bodyProgram,volumePanel,0,volumeMax,activeCount,true,viewStart,viewEnd);gl.drawArraysInstanced(gl.TRIANGLES,0,6,activeCount)}gl.useProgram(wickProgram);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);attrib(wickProgram,'aOpen',1,0);attrib(wickProgram,'aHigh',1,4);attrib(wickProgram,'aLow',1,8);attrib(wickProgram,'aClose',1,12);attrib(wickProgram,'aVolume',1,16);attrib(wickProgram,'aSourceStart',1,20);attrib(wickProgram,'aSourceEnd',1,24);uniforms(wickProgram,mainPanel,priceMin,priceMax,activeCount,false,viewStart,viewEnd);gl.drawArraysInstanced(gl.LINES,0,2,activeCount)}
async function tryWebGpu(){if(!navigator.gpu)return null;var adapter=await navigator.gpu.requestAdapter({powerPreference:'high-performance'});if(!adapter)return null;var device=await adapter.requestDevice();var context=gpu.getContext('webgpu');if(!context)return null;var format=navigator.gpu.getPreferredCanvasFormat();var dpr=Math.max(1,window.devicePixelRatio||1);gpu.width=__WIDTH__*dpr;gpu.height=__HEIGHT__*dpr;context.configure({device:device,format:format,alphaMode:'opaque'});var dataBuffer=device.createBuffer({size:Math.max(4,packed.byteLength),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});device.queue.writeBuffer(dataBuffer,0,packed);var uniformBuffer=device.createBuffer({size:96,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});var shaderModule=device.createShaderModule({code:`struct Params{size:vec2<f32>,pad0:vec2<f32>,panel:vec4<f32>,min:f32,max:f32,count:f32,barWidth:f32,mode:f32,upColor:vec4<f32>,downColor:vec4<f32>};@group(0)@binding(0)var<storage,read> values:array<f32>;@group(0)@binding(1)var<uniform> p:Params;struct Out{@builtin(position)pos:vec4<f32>,@location(0)up:f32};fn price(i:u32,o:u32)->f32{return values[i*5u+o]}fn clip(x:f32,y:f32)->vec4<f32>{let px=p.panel.x+x*p.panel.z;let py=p.panel.y+(1.0-y)*p.panel.w;return vec4<f32>(px/p.size.x*2.0-1.0,1.0-py/p.size.y*2.0,0.0,1.0)}@vertex fn vsBody(@builtin(vertex_index)vi:u32,@builtin(instance_index)i:u32)->Out{var v=array<vec2<f32>,6>(vec2<f32>(-1,-1),vec2<f32>(1,-1),vec2<f32>(1,1),vec2<f32>(-1,-1),vec2<f32>(1,1),vec2<f32>(-1,1));let o=price(i,0u);let h=price(i,1u);let l=price(i,2u);let c=price(i,3u);let vol=price(i,4u);let top=select(max(o,c),vol,p.mode>0.5);let bottom=select(min(o,c),0.0,p.mode>0.5);let value=mix(bottom,top,(v[vi].y+1.0)*0.5);let f=(value-p.min)/max(p.max-p.min,0.000001);let x=(f32(i)+0.5)/p.count+v[vi].x*p.barWidth;var r:Out;r.pos=clip(x,f);r.up=select(0.0,1.0,c>=o);return r}@vertex fn vsWick(@builtin(vertex_index)vi:u32,@builtin(instance_index)i:u32)->Out{let o=price(i,0u);let h=price(i,1u);let l=price(i,2u);let c=price(i,3u);let value=select(l,h,vi==1u);let f=(value-p.min)/max(p.max-p.min,0.000001);let x=(f32(i)+0.5)/p.count;var r:Out;r.pos=clip(x,f);r.up=select(0.0,1.0,c>=o);return r}@fragment fn fs(in:Out)->@location(0)vec4<f32>{return select(p.downColor,p.upColor,in.up>0.5)}`});var bodyPipeline=device.createRenderPipeline({layout:'auto',vertex:{module:shaderModule,entryPoint:'vsBody'},fragment:{module:shaderModule,entryPoint:'fs',targets:[{format:format}]},primitive:{topology:'triangle-list'}});var wickPipeline=device.createRenderPipeline({layout:'auto',vertex:{module:shaderModule,entryPoint:'vsWick'},fragment:{module:shaderModule,entryPoint:'fs',targets:[{format:format}]},primitive:{topology:'line-list'}});var bindGroup=device.createBindGroup({layout:bodyPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:dataBuffer}},{binding:1,resource:{buffer:uniformBuffer}}]});function params(panel,min,max,mode){var a=new Float32Array(24);a[0]=gpu.width;a[1]=gpu.height;a[4]=panel[0];a[5]=panel[1];a[6]=panel[2];a[7]=panel[3];a[8]=min;a[9]=max;a[10]=count;a[11]=Math.min(.82/count,.02);a[12]=mode?1:0;a.set(upColor,16);a.set(downColor,20);device.queue.writeBuffer(uniformBuffer,0,a)}function draw(){var encoder=device.createCommandEncoder();var pass=encoder.beginRenderPass({colorAttachments:[{view:context.getCurrentTexture().createView(),clearValue:{r:color(background)[0],g:color(background)[1],b:color(background)[2],a:1},loadOp:'clear',storeOp:'store'}]});pass.setBindGroup(0,bindGroup);pass.setPipeline(bodyPipeline);params(mainPanel,priceMin,priceMax,false);pass.draw(6,count);if(volumePanel){params(volumePanel,0,volumeMax,true);pass.draw(6,count)}pass.setPipeline(wickPipeline);params(mainPanel,priceMin,priceMax,false);pass.draw(2,count);pass.end();device.queue.submit([encoder.finish()])}draw();return draw}
function drawCpu(){selectActive();var ctx=gpu.getContext('2d');var dpr=Math.max(1,window.devicePixelRatio||1);gpu.width=__WIDTH__*dpr;gpu.height=__HEIGHT__*dpr;ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,__WIDTH__,__HEIGHT__);ctx.fillStyle=background;ctx.fillRect(0,0,__WIDTH__,__HEIGHT__);var span=Math.max(priceMax-priceMin,0.000001),visible=visibleSpan();for(var j=0;j<activeCount;j++){var base=j*7,s=activePacked[base+5],e=activePacked[base+6],center=(s+e)*.5;if(center<viewStart||center>viewEnd)continue;var x=mainPanel[0]+(center-viewStart)/visible*mainPanel[2],w=Math.max(1,(e-s)/visible*mainPanel[2]*.72),up=activePacked[base+3]>=activePacked[base];ctx.strokeStyle=up?'__UP_COLOR__':'__DOWN_COLOR__';ctx.fillStyle=ctx.strokeStyle;ctx.beginPath();ctx.moveTo(x,mainPanel[1]+(1-(activePacked[base+1]-priceMin)/span)*mainPanel[3]);ctx.lineTo(x,mainPanel[1]+(1-(activePacked[base+2]-priceMin)/span)*mainPanel[3]);ctx.stroke();var top=Math.max(activePacked[base],activePacked[base+3]),bottom=Math.min(activePacked[base],activePacked[base+3]);ctx.fillRect(x-w/2,mainPanel[1]+(1-(top-priceMin)/span)*mainPanel[3],w,Math.max(1,(top-bottom)/span*mainPanel[3]))} }
function panBy(dx){var delta=-dx/mainPanel[2]*visibleSpan();viewStart+=delta;viewEnd+=delta;redraw()}function zoomAt(x,factor){var oldSpan=visibleSpan(),nextSpan=Math.max(3,Math.min(count,oldSpan/factor)),ratio=Math.max(0,Math.min(1,(x-mainPanel[0])/mainPanel[2])),center=viewStart+ratio*oldSpan;viewStart=center-ratio*nextSpan;viewEnd=viewStart+nextSpan;redraw()}
function setViewport(start,end){viewStart=Number(start)||0;viewEnd=Number(end)||count;if(viewEnd<=viewStart)viewEnd=viewStart+1;redraw();return{start:viewStart,end:viewEnd,bucket:activeBucket,count:activeCount}}
 function ensureCapacity(required){required=Math.max(0,Math.floor(Number(required)||0));if(required<=capacity)return true;var next=capacity;while(next<required)next*=2;var nextPacked=new Float32Array(next*7);for(var i=0;i<count;i++){var src=physicalIndex(i)*7;nextPacked.set(packed.subarray(src,src+7),i*7)}packed=nextPacked;capacity=next;ringEnabled=false;ringHead=0;ringCapacity=0;if(gpuResize)gpuResize();return true}function reserve(extra){extra=Math.max(0,Math.floor(Number(extra)||0));if(ringEnabled)return ringCapacity;if(extra>0)ensureCapacity(count+extra);return capacity}function setRingBuffer(maxBars){var target=Math.floor(Number(maxBars)||0);if(target<=0){if(ringEnabled){var keep=count,next=1;while(next<Math.max(1024,keep*2))next*=2;var normal=new Float32Array(next*7);for(var i=0;i<keep;i++)normal.set(packed.subarray(physicalIndex(i)*7,physicalIndex(i)*7+7),i*7);packed=normal;capacity=next;ringEnabled=false;ringHead=0;ringCapacity=0;levelCache={};if(gpuResize)gpuResize();recalcBounds();redraw()}return capacity}target=Math.max(1,target);var keep=Math.min(count,target),nextPacked=new Float32Array(target*7),nextBars=bars.slice(count-keep);for(var i=0;i<keep;i++)nextPacked.set(packed.subarray(physicalIndex(count-keep+i)*7,physicalIndex(count-keep+i)*7+7),i*7);packed=nextPacked;bars=nextBars;count=keep;capacity=target;ringCapacity=target;ringHead=0;ringEnabled=true;viewStart=Math.max(0,Math.min(viewStart,count));viewEnd=count?Math.max(viewStart+1,Math.min(count,viewEnd)):0;levelCache={};if(gpuResize)gpuResize();recalcBounds();redraw();return ringCapacity}function updateBar(index,values){index=Math.floor(Number(index));if(index<0||index>=count||!values||values.length<5)return false;var physical=physicalIndex(index),base=physical*7;for(var k=0;k<5;k++)packed[base+k]=Number(values[k]);if(bars[index]){bars[index].open=packed[base];bars[index].high=packed[base+1];bars[index].low=packed[base+2];bars[index].close=packed[base+3];bars[index].volume=packed[base+4]}if(gpuRawUpdate)gpuRawUpdate(physical);levelCache={};recalcBounds();redraw();return true}function appendBar(date,values,timestamp){if(!values||values.length<5)return false;var wasTail=viewEnd>=count-0.5,oldSpan=visibleSpan(),index=count,physical;if(ringEnabled){if(count<ringCapacity){physical=physicalIndex(count);count++}else{physical=ringHead;ringHead=(ringHead+1)%ringCapacity;bars.shift()}}else{ensureCapacity(count+1);physical=count;count++}var sourceIndex=nextSourceIndex++;var base=physical*7;for(var k=0;k<5;k++)packed[base+k]=Number(values[k]);packed[base+5]=count-1;packed[base+6]=count;bars.push({index:sourceIndex,source_end:sourceIndex+1,date:String(date),timestamp:timestamp==null?null:Number(timestamp),open:packed[base],high:packed[base+1],low:packed[base+2],close:packed[base+3],volume:packed[base+4]});if(wasTail){viewEnd=count;viewStart=Math.max(0,count-oldSpan)}else{viewStart=Math.min(viewStart,Math.max(0,count-1));viewEnd=Math.min(viewEnd,count)}if(gpuRawUpdate)gpuRawUpdate(physical);levelCache={};recalcBounds();redraw();return true}
function tooltip(e){var r=gpu.getBoundingClientRect(),x=e.clientX-r.left,y=e.clientY-r.top;if(x<mainPanel[0]||x>mainPanel[0]+mainPanel[2]||y<mainPanel[1]||y>mainPanel[1]+mainPanel[3]){hover=null;drawOverlay();tip.style.display='none';return}if(dragging){panBy(x-dragX);dragX=x}hover={x:x,y:y};drawOverlay();var index=Math.max(0,Math.min(count-1,Math.floor(viewStart+(x-mainPanel[0])/(mainPanel[2]/visibleSpan())))),bar=bars[index],activeIndex=Math.max(0,Math.min(activeCount-1,Math.floor((index-activeSourceStart)/Math.max(1,activeBucket))));if(activeBucket>1&&activeBars[activeIndex])bar=activeBars[activeIndex];tip.innerHTML='<b>GPU 数据窗口 · '+esc(bar.date)+' #'+bar.index+'</b>'+(bar.timestamp==null?'':'<div>时间戳 '+esc(bar.timestamp)+'</div>')+'<div>开 '+bar.open.toFixed(2)+'　高 '+bar.high.toFixed(2)+'</div><div>低 '+bar.low.toFixed(2)+'　收 '+bar.close.toFixed(2)+'</div><div>成交量 '+bar.volume.toFixed(2)+'</div>'+(activeBucket>1?'<div>包络桶 ×'+activeBucket+'</div>':'');tip.style.display='block';tip.style.left=Math.max(6,Math.min(window.innerWidth-tip.offsetWidth-6,e.clientX+14))+'px';tip.style.top=Math.max(6,Math.min(window.innerHeight-tip.offsetHeight-6,e.clientY+14))+'px'}
drawOverlay();try{webgpuDraw=await tryWebGpu()}catch(error){webgpuDraw=null}if(webgpuDraw){ready=true;shell.dataset.renderer='webgpu'}else{try{if(!gl)throw new Error('WebGL2 unavailable');bodyProgram=program(vertexBody,fragment);wickProgram=program(vertexWick,fragment);buffer=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,packed,gl.STATIC_DRAW);bodyVertices=gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,bodyVertices);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,1,1,-1,-1,1,1,-1,1]),gl.STATIC_DRAW);drawGpu();ready=true;shell.dataset.renderer='webgl2'}catch(error){ready=false;fallback.style.display='block';drawCpu()}}drawOverlay();gpu.addEventListener('pointermove',tooltip);gpu.addEventListener('pointerleave',function(){tip.style.display='none'});window.addEventListener('resize',function(){if(webgpuDraw){webgpuDraw()}else if(ready){drawGpu()}drawOverlay()});})();</script></body></html>"#.to_string();
        let replacements = [
            ("__TITLE__", title),
            ("__BACKGROUND__", background.clone()),
            ("__WIDTH__", config.width.to_string()),
            ("__HEIGHT__", config.height.to_string()),
            ("__BARS__", bars_json),
            ("__VALUES_B64__", values_b64),
            ("__MAIN_PANEL__", main_panel_json),
            ("__VOLUME_PANEL__", volume_panel_json),
            ("__PRICE_MIN__", price_min.to_string()),
            ("__PRICE_MAX__", price_max.to_string()),
            ("__VOLUME_MAX__", volume_max.to_string()),
            ("__DYNAMIC_LOD__", dynamic_lod.to_string()),
            ("__OVERLAY__", overlay_json),
            ("__INDICATORS__", indicators_json),
            ("__HITS__", hit_regions_json),
            (
                "__INTERACTION_ENABLED__",
                if config.interaction.enabled {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
            (
                "__INTERACTION_CROSSHAIR__",
                if config.interaction.show_crosshair {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
            (
                "__INTERACTION_DATA_WINDOW__",
                if config.interaction.show_data_window {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
            (
                "__INTERACTION_PAN_ZOOM__",
                if config.interaction.enable_pan_zoom {
                    "true".to_string()
                } else {
                    "false".to_string()
                },
            ),
            ("__UP_COLOR__", up_color.to_string()),
            ("__DOWN_COLOR__", down_color.to_string()),
        ];
        for (needle, value) in replacements {
            html = html.replace(needle, &value);
        }
        html = html.replace(
            "gpu.addEventListener('pointerleave',function(){tip.style.display='none'})",
            "gpu.addEventListener('pointerleave',function(){hover=null;tip.style.display='none';drawOverlay()})",
        );
        html = html.replace(
            "gpu.addEventListener('pointermove',tooltip);",
            "gpu.addEventListener('pointerdown',function(e){if(!interaction.enabled||!interaction.panZoom)return;var r=gpu.getBoundingClientRect(),x=e.clientX-r.left;if(x>=mainPanel[0]&&x<=mainPanel[0]+mainPanel[2]){dragging=true;dragX=x;if(gpu.setPointerCapture)gpu.setPointerCapture(e.pointerId)}});gpu.addEventListener('pointerup',function(){dragging=false});gpu.addEventListener('pointercancel',function(){dragging=false});gpu.addEventListener('wheel',function(e){if(!interaction.enabled||!interaction.panZoom)return;var r=gpu.getBoundingClientRect(),x=e.clientX-r.left,y=e.clientY-r.top;if(x>=mainPanel[0]&&x<=mainPanel[0]+mainPanel[2]&&y>=mainPanel[1]&&y<=mainPanel[1]+mainPanel[3]){e.preventDefault();zoomAt(x,e.deltaY<0?1.25:0.8)}},{passive:false});gpu.addEventListener('pointermove',enhancedTooltip);",
        );
        let enhanced_tooltip = r#"function finiteNumber(v){return v!==null&&v!==undefined&&Number.isFinite(Number(v))}function fmtNumber(v,d){return finiteNumber(v)?Number(v).toFixed(d||2):'--'}function signedNumber(v,d){return finiteNumber(v)?(Number(v)>=0?'+':'')+Number(v).toFixed(d||2):'--'}function hitAt(x,y){var span=visibleSpan(),localX=mainPanel[0]+(x-mainPanel[0])*span/Math.max(count,1)+viewStart*mainPanel[2]/Math.max(count,1);return hitRegions.filter(function(item){return localX>=item.x&&localX<=item.x+item.width&&y>=item.y&&y<=item.y+item.height}).sort(function(a,b){return b.priority-a.priority})[0]}function enhancedTooltip(e){var r=gpu.getBoundingClientRect(),x=e.clientX-r.left,y=e.clientY-r.top;if(!interaction.enabled||x<mainPanel[0]||x>mainPanel[0]+mainPanel[2]||y<mainPanel[1]||y>mainPanel[1]+mainPanel[3]){hover=null;drawOverlay();tip.style.display='none';return}if(dragging){panBy(x-dragX);dragX=x}hover={x:x,y:y};drawOverlay();if(!count){tip.style.display='none';return}var index=Math.max(0,Math.min(count-1,Math.floor(viewStart+(x-mainPanel[0])/(mainPanel[2]/visibleSpan())))),bar=bars[index];if(!bar){tip.style.display='none';return}var activeIndex=Math.max(0,Math.min(activeCount-1,Math.floor((index-activeSourceStart)/Math.max(1,activeBucket)))),displayBar=activeBucket>1&&activeBars[activeIndex]?activeBars[activeIndex]:bar,previous=index>0?bars[index-1]:null,change=previous&&finiteNumber(displayBar.close)&&finiteNumber(previous.close)?displayBar.close-previous.close:null,changePct=previous&&finiteNumber(change)&&Math.abs(Number(previous.close))>1e-12?change/previous.close*100:null,amplitude=finiteNumber(displayBar.high)&&finiteNumber(displayBar.low)&&Math.abs(Number(displayBar.low))>1e-12?Math.abs(displayBar.high-displayBar.low)/Math.abs(displayBar.low)*100:null,indicatorIndex=Math.max(0,Math.min((indicatorRows.length&&indicatorRows[0].values.length||count)-1,activeBucket>1?activeSourceStart+activeIndex*activeBucket:index)),hit=hitAt(x,y),content='<b>GPU 数据窗口 · '+esc(displayBar.date)+' #'+displayBar.index+(displayBar.source_end>displayBar.index+1?' ['+displayBar.index+','+(displayBar.source_end-1)+']':'')+'</b>'+(displayBar.timestamp==null?'':'<div>时间戳 '+esc(displayBar.timestamp)+'</div>')+'<div>开 '+fmtNumber(displayBar.open)+'</div><div>高 '+fmtNumber(displayBar.high)+'</div><div>低 '+fmtNumber(displayBar.low)+'</div><div>收 '+fmtNumber(displayBar.close)+'</div><div>昨收 '+fmtNumber(previous&&previous.close)+'</div><div>涨跌 '+signedNumber(change)+'</div><div>涨幅 '+signedNumber(changePct)+'%</div><div>振幅 '+fmtNumber(amplitude)+'%</div><div>成交量 '+fmtNumber(displayBar.volume)+'</div>'+(activeBucket>1?'<div>包络桶 ×'+activeBucket+'</div>':'');if(indicatorRows.length){content+='<div class="gpu-tooltip-section">指标</div>';indicatorRows.forEach(function(row){content+='<div>'+esc(row.name)+' '+fmtNumber(row.values[indicatorIndex])+'</div>'})}if(hit)content+='<div class="gpu-tooltip-section">'+esc(hit.tooltip||hit.target)+'</div>';if(interaction.dataWindow){tip.innerHTML=content;tip.style.display='block'}else tip.style.display='none';tip.style.left=Math.max(6,Math.min(window.innerWidth-tip.offsetWidth-6,e.clientX+14))+'px';tip.style.top=Math.max(6,Math.min(window.innerHeight-tip.offsetHeight-6,e.clientY+14))+'px'}"#;
        let gpu_bootstrap = if prefer_webgpu {
            "drawOverlay();try{if(true){webgpuDraw=await tryWebGpu()}}"
        } else {
            "drawOverlay();try{if(false){webgpuDraw=await tryWebGpu()}}"
        };
        let tooltip_bootstrap = format!("{enhanced_tooltip}{gpu_bootstrap}");
        html = html
            .replace(
                "drawOverlay();try{webgpuDraw=await tryWebGpu()}",
                &tooltip_bootstrap,
            )
            .replace(
                "async function tryWebGpu(){",
                "function recoverWebGl(){try{if(!gl)throw new Error('WebGL2 unavailable');bodyProgram=bodyProgram||program(vertexBody,fragment);wickProgram=wickProgram||program(vertexWick,fragment);buffer=buffer||gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,packed,gl.STATIC_DRAW);bodyVertices=bodyVertices||gl.createBuffer();gl.bindBuffer(gl.ARRAY_BUFFER,bodyVertices);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,1,1,-1,-1,1,1,-1,1]),gl.STATIC_DRAW);ready=true;shell.dataset.renderer='webgl2';shell.dataset.gpuRecovery='webgl2';fallback.style.display='none';drawGpu()}catch(error){ready=false;shell.dataset.renderer='canvas2d';shell.dataset.gpuRecovery='canvas2d';fallback.style.display='block';drawCpu()}}async function tryWebGpu(){",
            )
            .replace(
                "var device=await adapter.requestDevice();",
                "var device=await adapter.requestDevice();device.lost.then(function(){if(webgpuDraw||shell.dataset.renderer==='webgpu'){shell.dataset.gpuLost='true';webgpuDraw=null;gpuRawUpdate=null;gpuResize=null;recoverWebGl()}});",
            )
            .replace(
                "var dataBuffer=device.createBuffer({size:Math.max(4,packed.byteLength),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});device.queue.writeBuffer(dataBuffer,0,packed);",
                "var dataBuffer=device.createBuffer({size:Math.max(4,packed.byteLength),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});var rawBuffer=device.createBuffer({size:Math.max(4,packed.byteLength),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});device.queue.writeBuffer(rawBuffer,0,packed);",
            )
            .replace("i*5u+o", "i*7u+o")
            .replace(
                "mode:f32,upColor:vec4<f32>",
                "mode:f32,start:f32,end:f32,upColor:vec4<f32>",
            )
            .replace(
                "let x=(f32(i)+0.5)/p.count+v[vi].x*p.barWidth",
                "let sourceStart=price(i,5u);let sourceEnd=price(i,6u);let span=max(p.end-p.start,1.0);let center=(sourceStart+sourceEnd)*0.5;let width=max(sourceEnd-sourceStart,1.0)/span*p.barWidth;let x=(center-p.start)/span+v[vi].x*width",
            )
            .replace(
                "let x=(f32(i)+0.5)/p.count;",
                "let sourceStart=price(i,5u);let sourceEnd=price(i,6u);let span=max(p.end-p.start,1.0);let x=((sourceStart+sourceEnd)*0.5-p.start)/span;",
            )
            .replace(
                "a[11]=Math.min(.82/count,.02);",
                "a[11]=.82;a[13]=viewStart;a[14]=viewEnd;",
            )
            .replace("a[10]=count;", "a[10]=activeCount;")
            .replace(
                "function draw(){var encoder=",
                "function draw(){selectActive();var encoder=",
            )
            .replace(
                "var pass=encoder.beginRenderPass({colorAttachments:",
                "if(dynamicLod&&activeBucket>1&&computePipeline){device.queue.writeBuffer(computeUniformBuffer,0,new Uint32Array([activeSourceStart,Math.min(count,activeSourceStart+activeCount*activeBucket),activeBucket,activeCount,ringHead,ringEnabled?ringCapacity:capacity]));var computePass=encoder.beginComputePass();computePass.setPipeline(computePipeline);computePass.setBindGroup(0,computeBindGroup);computePass.dispatchWorkgroups(Math.ceil(activeCount/64));computePass.end()}else{device.queue.writeBuffer(dataBuffer,0,activePacked)}var pass=encoder.beginRenderPass({colorAttachments:",
            )
            .replace(
                "var bindGroup=device.createBindGroup({layout:bodyPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:dataBuffer}},{binding:1,resource:{buffer:uniformBuffer}}]});function params(",
                "var bindGroup=device.createBindGroup({layout:bodyPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:dataBuffer}},{binding:1,resource:{buffer:uniformBuffer}}]});var computeModule=device.createShaderModule({code:`struct Reduce{start:u32,end:u32,bucket:u32,count:u32,head:u32,capacity:u32};@group(0)@binding(0)var<storage,read> raw:array<f32>;@group(0)@binding(1)var<storage,read_write> output:array<f32>;@group(0)@binding(2)var<uniform> p:Reduce;fn rv(i:u32,o:u32)->f32{let physical=(p.head+i)%p.capacity;return raw[physical*7u+o]}@compute @workgroup_size(64)fn main(@builtin(global_invocation_id) id:vec3<u32>){let b=id.x;if(b>=p.count){return;}let s=p.start+b*p.bucket;let e=min(p.end,s+p.bucket);if(s>=e){return;}var hi=rv(s,1u);var lo=rv(s,2u);var volume=0.0;var i=s;loop{if(i>=e){break;}hi=max(hi,rv(i,1u));lo=min(lo,rv(i,2u));volume=volume+rv(i,4u);i=i+1u;}let o=b*7u;output[o]=rv(s,0u);output[o+1u]=hi;output[o+2u]=lo;output[o+3u]=rv(e-1u,3u);output[o+4u]=volume;output[o+5u]=f32(s);output[o+6u]=f32(e)}}`});var computePipeline=device.createComputePipeline({layout:'auto',compute:{module:computeModule,entryPoint:'main'}});var computeUniformBuffer=device.createBuffer({size:24,usage:GPUBufferUsage.UNIFORM|GPUBufferUsage.COPY_DST});var computeBindGroup=device.createBindGroup({layout:computePipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:rawBuffer}},{binding:1,resource:{buffer:dataBuffer}},{binding:2,resource:{buffer:computeUniformBuffer}}]});gpuRawUpdate=function(index){device.queue.writeBuffer(rawBuffer,index*28,packed.subarray(index*7,index*7+7))};gpuResize=function(){dataBuffer.destroy();rawBuffer.destroy();dataBuffer=device.createBuffer({size:Math.max(4,packed.byteLength),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});rawBuffer=device.createBuffer({size:Math.max(4,packed.byteLength),usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});device.queue.writeBuffer(rawBuffer,0,packed);bindGroup=device.createBindGroup({layout:bodyPipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:dataBuffer}},{binding:1,resource:{buffer:uniformBuffer}}]});computeBindGroup=device.createBindGroup({layout:computePipeline.getBindGroupLayout(0),entries:[{binding:0,resource:{buffer:rawBuffer}},{binding:1,resource:{buffer:dataBuffer}},{binding:2,resource:{buffer:computeUniformBuffer}}]})};function params(",
            )
            .replace("pass.draw(6,count)", "pass.draw(6,activeCount)")
            .replace("pass.draw(2,count)", "pass.draw(2,activeCount)")
            .replace(
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.STATIC_DRAW)",
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW)",
            )
            .replace(
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW);bodyVertices=",
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW);gpuUpload=function(values){gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferSubData(gl.ARRAY_BUFFER,0,values)};gpuResize=function(){gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW)};bodyVertices=",
            )
            .replace(
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.STATIC_DRAW)",
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW)",
            )
            .replace(
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW);bodyVertices=",
                "gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW);gpuUpload=function(values){gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferSubData(gl.ARRAY_BUFFER,0,values)};gpuResize=function(){gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,packed,gl.DYNAMIC_DRAW)};bodyVertices=",
            )
            .replace(
                "window.addEventListener('resize',function(){if(webgpuDraw){webgpuDraw()}else if(ready){drawGpu()}drawOverlay()});})();",
                "window.addEventListener('resize',function(){if(webgpuDraw){webgpuDraw()}else if(ready){drawGpu()}drawOverlay()});window.__finkitGpuChart={setViewport:setViewport,updateBar:updateBar,appendBar:appendBar,reserve:reserve,setRingBuffer:setRingBuffer,panBy:panBy,zoomAt:zoomAt,getState:function(){return{start:viewStart,end:viewEnd,bucket:activeBucket,activeStart:activeSourceStart,activeCount:activeCount,rawCount:count,capacity:capacity,ringEnabled:ringEnabled,ringHead:ringHead,ringCapacity:ringCapacity,gpuLost:shell.dataset.gpuLost==='true',gpuRecovery:shell.dataset.gpuRecovery||null,renderer:shell.dataset.renderer||'canvas'}}};shell.dataset.controller='ready';})();",
            );
        // Pointer movement redraws the transparent overlay at high frequency.
        // Keep its backing store when the pixel ratio is unchanged; assigning
        // width/height on every hover event clears the context and allocates a
        // new backing buffer unnecessarily.
        html = html
            .replace(
                "var hover=null;var viewStart=0;var viewEnd=count;",
                "var overlayPixelWidth=0;var overlayPixelHeight=0;var hover=null;var viewStart=0;var viewEnd=count;",
            )
            .replace(
                &format!("var dpr=Math.max(1,window.devicePixelRatio||1);overlay.width={}*dpr;overlay.height={}*dpr;ctx.setTransform(dpr,0,0,dpr,0,0);", config.width, config.height),
                &format!("var dpr=Math.max(1,window.devicePixelRatio||1),pixelWidth={}*dpr,pixelHeight={}*dpr;if(overlayPixelWidth!==pixelWidth||overlayPixelHeight!==pixelHeight){{overlay.width=pixelWidth;overlay.height=pixelHeight;overlayPixelWidth=pixelWidth;overlayPixelHeight=pixelHeight}}ctx.setTransform(dpr,0,0,dpr,0,0);", config.width, config.height),
            )
            .replace(
                &format!("var dpr=Math.max(1,window.devicePixelRatio||1);gpu.width={}*dpr;gpu.height={}*dpr;gl.viewport(0,0,gpu.width, gpu.height);", config.width, config.height),
                &format!("var dpr=Math.max(1,window.devicePixelRatio||1),pixelWidth={}*dpr,pixelHeight={}*dpr;if(gpu.width!==pixelWidth||gpu.height!==pixelHeight){{gpu.width=pixelWidth;gpu.height=pixelHeight}}gl.viewport(0,0,gpu.width, gpu.height);", config.width, config.height),
            )
            .replace(
                &format!("var dpr=Math.max(1,window.devicePixelRatio||1);gpu.width={}*dpr;gpu.height={}*dpr;ctx.setTransform(dpr,0,0,dpr,0,0);", config.width, config.height),
                &format!("var dpr=Math.max(1,window.devicePixelRatio||1),pixelWidth={}*dpr,pixelHeight={}*dpr;if(gpu.width!==pixelWidth||gpu.height!==pixelHeight){{gpu.width=pixelWidth;gpu.height=pixelHeight}}ctx.setTransform(dpr,0,0,dpr,0,0);", config.width, config.height),
            )
            // Updating a live bar should be O(1) in the common case. A full
            // bounds scan is only needed when an old extremum is replaced or
            // evicted from a ring buffer.
            .replace(
                "function recalcBounds(){priceMin=Infinity;priceMax=-Infinity;volumeMax=1;for(var i=0;i<count;i++){priceMin=Math.min(priceMin,rawValue(i,2));priceMax=Math.max(priceMax,rawValue(i,1));var volume=rawValue(i,4);volumeMax=Math.max(volumeMax,Number.isFinite(volume)?volume:0)}}",
                "function recalcBounds(){priceMin=Infinity;priceMax=-Infinity;volumeMax=1;for(var i=0;i<count;i++){priceMin=Math.min(priceMin,rawValue(i,2));priceMax=Math.max(priceMax,rawValue(i,1));var volume=rawValue(i,4);volumeMax=Math.max(volumeMax,Number.isFinite(volume)?volume:0)}}function updateBounds(oldLow,oldHigh,oldVolume,newLow,newHigh,newVolume,removed){if(!Number.isFinite(newLow)||!Number.isFinite(newHigh)||!Number.isFinite(newVolume)){recalcBounds();return}var oldExtremum=(Number.isFinite(oldLow)&&oldLow===priceMin&&newLow>oldLow)||(Number.isFinite(oldHigh)&&oldHigh===priceMax&&newHigh<oldHigh)||(Number.isFinite(oldVolume)&&oldVolume===volumeMax&&newVolume<oldVolume);if((removed||Number.isFinite(oldLow)||Number.isFinite(oldHigh)||Number.isFinite(oldVolume))&&oldExtremum){recalcBounds();return}priceMin=Math.min(priceMin,newLow);priceMax=Math.max(priceMax,newHigh);volumeMax=Math.max(volumeMax,newVolume)}",
            )
            .replace(
                "var physical=physicalIndex(index),base=physical*7;for(var k=0;k<5;k++)packed[base+k]=Number(values[k]);",
                "var physical=physicalIndex(index),base=physical*7,oldLow=packed[base+2],oldHigh=packed[base+1],oldVolume=packed[base+4];for(var k=0;k<5;k++)packed[base+k]=Number(values[k]);",
            )
            .replace(
                "levelCache={};recalcBounds();redraw();return true}function appendBar",
                "levelCache={};updateBounds(oldLow,oldHigh,oldVolume,packed[base+2],packed[base+1],packed[base+4],false);redraw();return true}function appendBar",
            )
            .replace(
                "var wasTail=viewEnd>=count-0.5,oldSpan=visibleSpan(),index=count,physical;",
                "var wasTail=viewEnd>=count-0.5,oldSpan=visibleSpan(),index=count,physical,oldLow=NaN,oldHigh=NaN,oldVolume=NaN,removed=false;",
            )
            .replace(
                "physical=ringHead;ringHead=(ringHead+1)%ringCapacity;bars.shift()",
                "physical=ringHead;oldLow=packed[physical*7+2];oldHigh=packed[physical*7+1];oldVolume=packed[physical*7+4];removed=true;ringHead=(ringHead+1)%ringCapacity;bars.shift()",
            )
            .replace(
                "levelCache={};recalcBounds();redraw();return true}",
                "levelCache={};updateBounds(oldLow,oldHigh,oldVolume,packed[base+2],packed[base+1],packed[base+4],removed);redraw();return true}",
            );
        Ok(html)
    }
}

impl Default for WebGlRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for WebGlRenderer {
    fn render(&self, draw_list: &DrawList, config: &ChartConfig) -> Result<String> {
        let commands = CanvasRenderer::command_json(draw_list)?;
        let data = KlineData::new(
            vec!["".to_string()],
            vec![0.0],
            vec![1.0],
            vec![-1.0],
            vec![0.0],
            vec![0.0],
        );
        let scene = ChartScene::default();
        let mut html = self.render_with_data(
            config,
            &data,
            0,
            &[(0, 1)],
            Rect::new(0.0, 0.0, config.width as f64, config.height as f64),
            None,
            &scene,
            draw_list,
        )?;
        html = html.replace("__UNUSED_COMMANDS__", &commands);
        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChartConfig, IndicatorType};
    use crate::scene::{HitRegion, HitTarget};

    #[test]
    fn emits_instanced_webgl2_backend_and_cpu_fallback() {
        let data = KlineData::new(
            vec!["2026-01-01".into(), "2026-01-02".into()],
            vec![10.0, 11.0],
            vec![11.0, 12.0],
            vec![9.0, 10.0],
            vec![10.5, 11.5],
            vec![100.0, 120.0],
        );
        let html = WebGlRenderer::new()
            .render_with_data(
                &ChartConfig::default(),
                &data,
                0,
                &[(0, 1), (1, 2)],
                Rect::new(80.0, 20.0, 700.0, 300.0),
                None,
                &ChartScene::default(),
                &DrawList::new(),
            )
            .expect("WebGL output should render");
        assert!(html.contains("data-renderer=\"webgl2\""));
        assert!(html.contains("drawArraysInstanced"));
        assert!(html.contains("WebGL2 unavailable"));
        assert!(html.contains("finkit-webgl-tooltip"));
        assert!(html.contains("makeEnvelope"));
        assert!(html.contains("aSourceStart"));
        assert!(html.contains("bufferSubData"));
        assert!(html.contains("__finkitGpuChart") || html.contains("setViewport:setViewport"));
        assert!(html.contains("var dynamicLod=true"));
        assert!(html.contains("decodeF32"));
        assert!(html.contains("data-packed-format=\"f32-base64\""));
        assert!(html.contains("@compute @workgroup_size(64)"));
        assert!(html.contains("dispatchWorkgroups"));
        assert!(html.contains("appendBar:appendBar"));
        assert!(html.contains("capacity:capacity"));
        assert!(html.contains("setRingBuffer:setRingBuffer"));
        assert!(html.contains("device.lost"));
        assert!(html.contains("recoverWebGl"));
        assert!(html.contains("gpuRecovery"));
        assert!(html.contains("overlayPixelWidth"));
        assert!(html.contains("if(gpu.width!==pixelWidth||gpu.height!==pixelHeight)"));
    }

    #[test]
    fn embeds_indicator_and_semantic_hit_payload_for_gpu_tooltip() {
        let data = KlineData::new(
            vec!["2026-01-01".into(), "2026-01-02".into()],
            vec![10.0, 11.0],
            vec![11.0, 12.0],
            vec![9.0, 10.0],
            vec![10.5, 11.5],
            vec![100.0, 120.0],
        );
        let mut scene = ChartScene::default();
        scene.add_hit_region(HitRegion {
            rect: Rect::new(80.0, 20.0, 40.0, 300.0),
            target: HitTarget::ChanSignal {
                index: 1,
                kind: "B1".into(),
            },
            priority: 20,
            tooltip: Some("B1 candidate".into()),
        });
        let html = WebGlRenderer::new()
            .render_with_layers_and_indicators(
                &ChartConfig::default(),
                &data,
                0,
                &[(0, 1), (1, 2)],
                Rect::new(80.0, 20.0, 700.0, 300.0),
                None,
                &scene,
                &DrawList::new(),
                &DrawList::new(),
                &DrawList::new(),
                &[IndicatorConfig::new(IndicatorType::MA, vec![2.0])],
                &HashMap::new(),
            )
            .expect("WebGL output should include tooltip payloads");
        assert!(html.contains("\"name\":\"MA2\""));
        assert!(html.contains("\"tooltip\":\"B1 candidate\""));
        assert!(html.contains("var indicatorRows="));
        assert!(html.contains("var hitRegions="));
        assert!(html.contains("enhancedTooltip"));
    }
}
