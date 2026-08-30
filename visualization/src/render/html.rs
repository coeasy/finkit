use crate::config::ChartConfig;
use crate::error::Result;
use crate::primitive::DrawList;

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
             .chart-container svg{{display:block;cursor:crosshair}}\n\
             .chart-container svg .chart-content{{transition:none}}\n\
             .crosshair-h{{stroke:{cross};stroke-width:1;stroke-dasharray:4,2;pointer-events:none}}\n\
             .crosshair-v{{stroke:{cross};stroke-width:1;stroke-dasharray:4,2;pointer-events:none}}\n\
             .tooltip{{position:absolute;top:8px;left:8px;background:rgba(0,0,0,0.75);color:#fff;padding:6px 10px;border-radius:4px;font-size:12px;pointer-events:none;white-space:pre;line-height:1.5;z-index:10}}\n\
             </style>\n\
             </head>\n\
             <body>\n\
             <div class=\"chart-container\">\n\
             {svg}\n\
             <div class=\"tooltip\" id=\"tooltip\" style=\"display:none\"></div>\n\
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
            js = Self::js_interaction_layer(),
        );

        Ok(html)
    }
}

impl HtmlRenderer {
    fn js_interaction_layer() -> &'static str {
        r#"(function(){
var svg=document.querySelector('svg');
if(!svg)return;
var state={scaleX:1,offsetX:0,dragging:false,lastX:0,lastY:0};
var chH=null,chV=null,tooltip=null;
function init(){
  var g=svg.querySelector('.chart-content');
  if(!g){
    g=document.createElementNS('http://www.w3.org/2000/svg','g');
    g.setAttribute('class','chart-content');
    while(svg.childNodes.length>1){
      var c=svg.childNodes[1];
      if(c.nodeType===1&&c.tagName!=='rect')g.appendChild(c);
      else break;
    }
    svg.appendChild(g);
  }
  chH=document.createElementNS('http://www.w3.org/2000/svg','line');
  chH.setAttribute('class','crosshair-h');
  chH.setAttribute('x1','0');chH.setAttribute('y1','0');
  chH.setAttribute('x2',svg.getAttribute('width'));chH.setAttribute('y2','0');
  chH.style.display='none';svg.appendChild(chH);
  chV=document.createElementNS('http://www.w3.org/2000/svg','line');
  chV.setAttribute('class','crosshair-v');
  chV.setAttribute('x1','0');chV.setAttribute('y1','0');
  chV.setAttribute('x2','0');chV.setAttribute('y2',svg.getAttribute('height'));
  chV.style.display='none';svg.appendChild(chV);
  tooltip=document.getElementById('tooltip');
}
function applyTransform(){
  var g=svg.querySelector('.chart-content');
  if(g)g.setAttribute('transform','translate('+state.offsetX+',0) scale('+state.scaleX+',1)');
}
svg.addEventListener('wheel',function(e){
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
svg.addEventListener('mousedown',function(e){
  state.dragging=true;state.lastX=e.clientX;state.lastY=e.clientY;
  svg.style.cursor='grabbing';
});
svg.addEventListener('mousemove',function(e){
  if(state.dragging){
    state.offsetX+=e.clientX-state.lastX;
    state.lastX=e.clientX;state.lastY=e.clientY;
    applyTransform();
  }
  updateCrosshair(e);
});
svg.addEventListener('mouseup',function(){
  state.dragging=false;svg.style.cursor='crosshair';
});
svg.addEventListener('mouseleave',function(){
  state.dragging=false;svg.style.cursor='crosshair';hideCrosshair();
});
function updateCrosshair(e){
  var rect=svg.getBoundingClientRect();
  var x=e.clientX-rect.left;
  var y=e.clientY-rect.top;
  if(chH){chH.setAttribute('y1',y);chH.setAttribute('y2',y);chH.style.display='';}
  if(chV){chV.setAttribute('x1',x);chV.setAttribute('x2',x);chV.style.display='';}
  if(tooltip){
    tooltip.style.display='block';
    tooltip.textContent='X:'+Math.round(x)+' Y:'+Math.round(y);
  }
}
function hideCrosshair(){
  if(chH)chH.style.display='none';
  if(chV)chV.style.display='none';
  if(tooltip)tooltip.style.display='none';
}
init();
})();"#
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
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
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
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("<style>"));
        assert!(result.contains("</style>"));
        assert!(result.contains(".chart-container"));
    }

    #[test]
    fn test_html_contains_js() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("<script>"));
        assert!(result.contains("</script>"));
        assert!(result.contains("addEventListener"));
        assert!(result.contains("wheel"));
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
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("<line"));
        assert!(result.contains("x1=\"0.00\""));
    }

    #[test]
    fn test_html_contains_tooltip() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("tooltip"));
        assert!(result.contains("id=\"tooltip\""));
    }

    #[test]
    fn test_html_contains_crosshair_css() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains(".crosshair-h"));
        assert!(result.contains(".crosshair-v"));
    }

    #[test]
    fn test_html_meta_charset() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("charset=\"utf-8\""));
    }

    #[test]
    fn test_html_title_escaped() {
        let renderer = HtmlRenderer::new();
        let mut config = default_config();
        config.title = "<script>alert('xss')</script>".to_string();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>alert('xss')</script>"));
    }

    #[test]
    fn test_html_self_contained() {
        let renderer = HtmlRenderer::new();
        let config = default_config();
        let draw_list = DrawList::new();
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
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
        let result = renderer.render(&draw_list, &config).expect("alpha-ta-visualization: unexpected None/Err in visualization/src/render/html.rs (A5 governance)");
        assert!(result.contains(&config.theme_config.background_color));
        assert!(result.contains(&config.theme_config.crosshair_color));
    }

    #[test]
    fn test_html_js_size_under_5kb() {
        let js = HtmlRenderer::js_interaction_layer();
        assert!(
            js.len() < 5 * 1024,
            "JS interaction layer should be under 5KB, got {} bytes",
            js.len()
        );
    }
}
