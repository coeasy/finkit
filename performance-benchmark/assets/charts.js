// assets/charts.js — AlphaTA Performance Benchmark Charts
(function() {
  'use strict';

  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var green = style.getPropertyValue('--green').trim();
  var orange = style.getPropertyValue('--orange').trim();
  var red = style.getPropertyValue('--red').trim();

  var palette = [accent, accent2, green, orange, '#6366f1', '#ec4899', '#14b8a6', '#f97316'];

  // ============================================================
  // DATA
  // ============================================================
  var indicators = [
    { name: 'SMA(20)',       alpha: 0.019, talib: 0.021, ratio: 0.900 },
    { name: 'EMA(20)',       alpha: 0.025, talib: 0.029, ratio: 0.887 },
    { name: 'WMA(20)',       alpha: 0.025, talib: 0.021, ratio: 1.194 },
    { name: 'DEMA(20)',      alpha: 0.046, talib: 0.060, ratio: 0.769 },
    { name: 'TEMA(20)',      alpha: 0.069, talib: 0.090, ratio: 0.771 },
    { name: 'RSI(14)',       alpha: 0.030, talib: 0.055, ratio: 0.550 },
    { name: 'MACD(12,26,9)', alpha: 0.098, talib: 0.093, ratio: 1.053 },
    { name: 'MOM(10)',       alpha: 0.004, talib: 0.006, ratio: 0.683 },
    { name: 'ROC(10)',       alpha: 0.007, talib: 0.011, ratio: 0.598 },
    { name: 'CMO(14)',       alpha: 0.039, talib: 0.057, ratio: 0.684 },
    { name: 'TRIX(14)',      alpha: 0.074, talib: 0.115, ratio: 0.638 },
    { name: 'ATR(14)',       alpha: 0.030, talib: 0.059, ratio: 0.519 },
    { name: 'NATR(14)',      alpha: 0.041, talib: 0.059, ratio: 0.704 },
    { name: 'OBV',           alpha: 0.012, talib: 0.030, ratio: 0.405 },
    { name: 'BBANDS(20,2)',  alpha: 0.048, talib: 0.053, ratio: 0.907 },
    { name: 'STOCH(14,3,3)', alpha: 0.109, talib: 0.147, ratio: 0.740 },
    { name: 'CCI(14)',       alpha: 0.113, talib: 0.203, ratio: 0.556 },
    { name: 'WILLR(14)',     alpha: 0.182, talib: 0.097, ratio: 1.864 },
    { name: 'ADX(14)',       alpha: 0.068, talib: 0.085, ratio: 0.802 },
    { name: 'STDDEV(20)',    alpha: 0.023, talib: 0.036, ratio: 0.627 },
    { name: 'VAR(20)',       alpha: 0.019, talib: 0.022, ratio: 0.848 }
  ];

  var memoryData = [
    { name: 'SMA(20)',       alpha: 78.2,  talib: 156.2 },
    { name: 'EMA(20)',       alpha: 78.2,  talib: 156.2 },
    { name: 'WMA(20)',       alpha: 78.2,  talib: 156.2 },
    { name: 'DEMA(20)',      alpha: 156.2, talib: 234.4 },
    { name: 'TEMA(20)',      alpha: 156.2, talib: 234.4 },
    { name: 'RSI(14)',       alpha: 156.2, talib: 234.4 },
    { name: 'MACD(12,26,9)', alpha: 234.4, talib: 312.5 },
    { name: 'MOM(10)',       alpha: 78.1,  talib: 156.2 },
    { name: 'ROC(10)',       alpha: 78.1,  talib: 156.2 },
    { name: 'CCI(14)',       alpha: 78.5,  talib: 156.2 },
    { name: 'WILLR(14)',     alpha: 78.1,  talib: 156.2 },
    { name: 'ADX(14)',       alpha: 234.4, talib: 312.5 },
    { name: 'TRIX(14)',      alpha: 234.4, talib: 312.5 },
    { name: 'BBANDS(20,2)',  alpha: 234.4, talib: 312.5 },
    { name: 'ATR(14)',       alpha: 78.1,  talib: 156.2 },
    { name: 'NATR(14)',      alpha: 78.1,  talib: 156.2 },
    { name: 'OBV',           alpha: 156.2, talib: 234.4 },
    { name: 'STDDEV(20)',    alpha: 78.1,  talib: 156.2 },
    { name: 'VAR(20)',       alpha: 78.1,  talib: 156.2 }
  ];

  var streamingData = [
    { name: 'SMA',       time: 2.74,  thru: 3.65,  cat: '重叠' },
    { name: 'EMA',       time: 2.34,  thru: 4.28,  cat: '重叠' },
    { name: 'RSI',       time: 2.43,  thru: 4.12,  cat: '动量' },
    { name: 'MACD',      time: 9.47,  thru: 1.06,  cat: '动量' },
    { name: 'BOLL',      time: 2.34,  thru: 4.27,  cat: '波动' },
    { name: 'ATR',       time: 4.42,  thru: 2.26,  cat: '波动' },
    { name: 'KDJ',       time: 0.266, thru: 37.7,  cat: '动量' },
    { name: 'ALMA',      time: 0.198, thru: 50.6,  cat: '重叠' },
    { name: 'CMF',       time: 0.071, thru: 140,   cat: '成交量' },
    { name: 'PVT',       time: 0.016, thru: 639,   cat: '成交量' },
    { name: 'Ehlers SS', time: 0.032, thru: 309,   cat: '滤波器' },
    { name: 'Ehlers IT', time: 0.033, thru: 299,   cat: '滤波器' }
  ];

  var precisionData = [
    { name: 'SMA(20)',       maxErr: 4.21e-12, status: 'PASS' },
    { name: 'SMA(50)',       maxErr: 8.75e-12, status: 'PASS' },
    { name: 'EMA(20)',       maxErr: 2.84e-14, status: 'PASS' },
    { name: 'EMA(50)',       maxErr: 4.26e-14, status: 'PASS' },
    { name: 'WMA(20)',       maxErr: 2.65e-08, status: 'PASS' },
    { name: 'RSI(14)',       maxErr: 5.68e-14, status: 'PASS' },
    { name: 'ATR(14)',       maxErr: 2.66e-15, status: 'PASS' },
    { name: 'NATR(14)',      maxErr: 5.82e-11, status: 'PASS' },
    { name: 'ADX(14)',       maxErr: 0,        status: 'PASS' },
    { name: 'CCI(14)',       maxErr: 2.11e-09, status: 'PASS' },
    { name: 'WILLR(14)',     maxErr: 1.42e-14, status: 'PASS' },
    { name: 'OBV',           maxErr: 0,        status: 'PASS' },
    { name: 'DEMA(20)',      maxErr: 5.12e-13, status: 'PASS' },
    { name: 'TEMA(20)',      maxErr: 1.14e-12, status: 'PASS' },
    { name: 'TRIX(14)',      maxErr: 8.10e-07, status: 'PASS' },
    { name: 'MOM(10)',       maxErr: 0,        status: 'PASS' },
    { name: 'ROC(10)',       maxErr: 2.27e-13, status: 'PASS' },
    { name: 'CMO(14)',       maxErr: 1.05e-13, status: 'PASS' },
    { name: 'STDDEV(20)',    maxErr: 2.39e-09, status: 'PASS' },
    { name: 'VAR(20)',       maxErr: 2.67e-09, status: 'PASS' },
    { name: 'MACD(12,26,9)', maxErr: 5.68e-14, status: 'PASS' },
    { name: 'BBANDS(20,2)',  maxErr: 2.36e-09, status: 'PASS' },
    { name: 'STOCH(14,3,3)', maxErr: 1.85e-12, status: 'PASS' }
  ];

  var optimizationData = [
    { item: 'RSI 8-wide SIMD',       impact: 45,  desc: 'RSI 快 45% vs TA-Lib' },
    { item: 'BBANDS 零拷贝',         impact: 75,  desc: '内存降 4×' },
    { item: 'VarNameCache 复用',     impact: 15,  desc: '消除 12 次/调用小对象分配' },
    { item: 'NATR 精度修复',         impact: 99,  desc: '1.69e-02 → 5.82e-11' },
    { item: 'ADX 算法重写',          impact: 100, desc: '4.05e-01 → 0.00e+00' },
    { item: 'MACD SMA 种子 + FMA',   impact: 99,  desc: '2.71e-01 → 5.68e-14' },
    { item: '流式指标',              impact: 60,  desc: '160 个流式指标 O(1) 更新' },
    { item: '零分配 API',            impact: 65,  desc: '35+ 个 _into 变体, 0 KB 峰值' }
  ];

  // ============================================================
  // CHART 1: Execution Time Comparison (grouped bar, log scale)
  // ============================================================
  var chart1 = echarts.init(document.getElementById('chart-time'), null, { renderer: 'svg' });
  chart1.setOption({
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      formatter: function(params) {
        var s = '<strong>' + params[0].axisValue + '</strong><br/>';
        params.forEach(function(p) {
          s += '<span style="display:inline-block;width:10px;height:10px;border-radius:50%;background:' + p.color + ';margin-right:4px;"></span> ';
          s += p.seriesName + ': <strong>' + p.value.toFixed(3) + '</strong> ms<br/>';
        });
        return s;
      }
    },
    legend: {
      data: ['AlphaTA', 'TA-Lib'],
      textStyle: { color: muted, fontSize: 12 },
      top: 0
    },
    grid: { left: 50, right: 20, top: 40, bottom: 80 },
    xAxis: {
      type: 'category',
      data: indicators.map(function(d) { return d.name; }),
      axisLabel: { rotate: 45, fontSize: 10, color: muted, interval: 0 },
      axisLine: { lineStyle: { color: rule } },
      axisTick: { alignWithLabel: true }
    },
    yAxis: {
      type: 'log',
      name: '执行时间 (ms)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 10 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } },
      min: 0.001,
      max: 1
    },
    series: [
      {
        name: 'AlphaTA',
        type: 'bar',
        data: indicators.map(function(d) { return d.alpha; }),
        itemStyle: { color: accent, borderRadius: [2, 2, 0, 0] },
        barWidth: '30%'
      },
      {
        name: 'TA-Lib',
        type: 'bar',
        data: indicators.map(function(d) { return d.talib; }),
        itemStyle: { color: accent2, borderRadius: [2, 2, 0, 0] },
        barWidth: '30%'
      }
    ],
    animation: false
  });
  window.addEventListener('resize', function() { chart1.resize(); });

  // ============================================================
  // CHART 2: Speedup Ratio Distribution
  // ============================================================
  var chart2 = echarts.init(document.getElementById('chart-speedup'), null, { renderer: 'svg' });
  chart2.setOption({
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      formatter: function(params) {
        var p = params[0];
        var val = p.value;
        var faster = val < 1 ? 'AlphaTA 快 ' + ((1 - val) * 100).toFixed(0) + '%' :
                     val > 1 ? 'TA-Lib 快 ' + ((val - 1) * 100).toFixed(0) + '%' : '持平';
        return '<strong>' + p.axisValue + '</strong><br/>加速比: <strong>' + val.toFixed(3) + '×</strong><br/>' + faster;
      }
    },
    grid: { left: 50, right: 30, top: 20, bottom: 80 },
    xAxis: {
      type: 'category',
      data: indicators.map(function(d) { return d.name; }),
      axisLabel: { rotate: 45, fontSize: 10, color: muted, interval: 0 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '加速比 (×)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 10 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } },
      min: 0
    },
    series: [{
      type: 'bar',
      data: indicators.map(function(d) {
        return {
          value: d.ratio,
          itemStyle: {
            color: d.ratio < 1 ? green : red,
            borderRadius: [2, 2, 0, 0]
          }
        };
      }),
      barWidth: '50%',
      markLine: {
        silent: true,
        symbol: 'none',
        lineStyle: { color: accent, type: 'dashed', width: 1.5 },
        data: [{ yAxis: 1, label: { formatter: '1.0× (基准)', color: accent, fontSize: 10 } }]
      }
    }],
    animation: false
  });
  window.addEventListener('resize', function() { chart2.resize(); });

  // ============================================================
  // CHART 3: Memory Usage Comparison (grouped bar)
  // ============================================================
  var chart3 = echarts.init(document.getElementById('chart-memory'), null, { renderer: 'svg' });
  chart3.setOption({
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      formatter: function(params) {
        var s = '<strong>' + params[0].axisValue + '</strong><br/>';
        params.forEach(function(p) {
          s += '<span style="display:inline-block;width:10px;height:10px;border-radius:50%;background:' + p.color + ';margin-right:4px;"></span> ';
          s += p.seriesName + ': <strong>' + p.value.toFixed(1) + '</strong> KB<br/>';
        });
        return s;
      }
    },
    legend: {
      data: ['AlphaTA', 'TA-Lib'],
      textStyle: { color: muted, fontSize: 12 },
      top: 0
    },
    grid: { left: 50, right: 20, top: 40, bottom: 80 },
    xAxis: {
      type: 'category',
      data: memoryData.map(function(d) { return d.name; }),
      axisLabel: { rotate: 45, fontSize: 10, color: muted, interval: 0 },
      axisLine: { lineStyle: { color: rule } }
    },
    yAxis: {
      type: 'value',
      name: '峰值内存 (KB)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 10 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    series: [
      {
        name: 'AlphaTA',
        type: 'bar',
        data: memoryData.map(function(d) { return d.alpha; }),
        itemStyle: { color: green, borderRadius: [2, 2, 0, 0] },
        barWidth: '30%'
      },
      {
        name: 'TA-Lib',
        type: 'bar',
        data: memoryData.map(function(d) { return d.talib; }),
        itemStyle: { color: accent2, borderRadius: [2, 2, 0, 0] },
        barWidth: '30%'
      }
    ],
    animation: false
  });
  window.addEventListener('resize', function() { chart3.resize(); });

  // ============================================================
  // CHART 4: Memory Category Distribution (pie)
  // ============================================================
  var chart4 = echarts.init(document.getElementById('chart-mem-category'), null, { renderer: 'svg' });
  chart4.setOption({
    tooltip: {
      trigger: 'item',
      appendToBody: true,
      formatter: function(params) {
        return '<strong>' + params.name + '</strong><br/>' +
               'AlphaTA: ' + params.value + ' KB<br/>' +
               '占比: ' + params.percent + '%';
      }
    },
    legend: {
      orient: 'vertical',
      right: 10,
      top: 'center',
      textStyle: { color: muted, fontSize: 11 }
    },
    series: [{
      type: 'pie',
      radius: ['40%', '70%'],
      center: ['35%', '50%'],
      avoidLabelOverlap: true,
      label: {
        show: false
      },
      emphasis: {
        label: { show: true, fontSize: 14, fontWeight: 'bold' },
        itemStyle: { shadowBlur: 10, shadowOffsetX: 0, shadowColor: 'rgba(0,0,0,0.2)' }
      },
      data: [
        { value: 234.4, name: '复杂指标 (MACD/ADX/BBANDS)', itemStyle: { color: accent } },
        { value: 156.2, name: '中等指标 (DEMA/TEMA/RSI/OBV)', itemStyle: { color: accent2 } },
        { value: 78.2,  name: '简单指标 (SMA/EMA/WMA/MOM)', itemStyle: { color: green } },
        { value: 0,     name: '零分配变体 (_into 系列)', itemStyle: { color: orange } }
      ]
    }],
    animation: false
  });
  window.addEventListener('resize', function() { chart4.resize(); });

  // ============================================================
  // CHART 5: Optimization Impact (horizontal bar)
  // ============================================================
  var chart5 = echarts.init(document.getElementById('chart-optimization'), null, { renderer: 'svg' });
  chart5.setOption({
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      formatter: function(params) {
        var p = params[0];
        var item = optimizationData[p.dataIndex];
        return '<strong>' + p.axisValue + '</strong><br/>' +
               '改进程度: ' + p.value + '%<br/>' +
               '<span style="color:' + muted + ';font-size:0.85em;">' + item.desc + '</span>';
      }
    },
    grid: { left: 140, right: 50, top: 20, bottom: 30 },
    xAxis: {
      type: 'value',
      name: '改进程度 (%)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 10 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } },
      max: 100
    },
    yAxis: {
      type: 'category',
      data: optimizationData.map(function(d) { return d.item; }),
      axisLabel: { color: ink, fontSize: 11, fontWeight: 600 },
      axisLine: { lineStyle: { color: rule } },
      axisTick: { show: false }
    },
    series: [{
      type: 'bar',
      data: optimizationData.map(function(d, i) {
        return {
          value: d.impact,
          itemStyle: {
            color: palette[i % palette.length],
            borderRadius: [0, 4, 4, 0]
          }
        };
      }),
      barWidth: '55%',
      label: {
        show: true,
        position: 'right',
        formatter: function(p) { return p.value + '%'; },
        color: muted,
        fontSize: 11,
        fontWeight: 600
      }
    }],
    animation: false
  });
  window.addEventListener('resize', function() { chart5.resize(); });

  // ============================================================
  // CHART 6: Streaming Indicators Throughput (horizontal bar)
  // ============================================================
  var chart6 = echarts.init(document.getElementById('chart-streaming'), null, { renderer: 'svg' });
  chart6.setOption({
    tooltip: {
      trigger: 'axis',
      appendToBody: true,
      formatter: function(params) {
        var p = params[0];
        var item = streamingData[p.dataIndex];
        return '<strong>' + p.axisValue + '</strong><br/>' +
               '吞吐量: <strong>' + item.thru.toFixed(1) + '</strong> Gelem/s<br/>' +
               '执行时间: <strong>' + item.time.toFixed(3) + '</strong> ms<br/>' +
               '类别: <span style="color:' + muted + ';">' + item.cat + '</span>';
      }
    },
    grid: { left: 100, right: 50, top: 20, bottom: 30 },
    xAxis: {
      type: 'value',
      name: '吞吐量 (Gelem/s)',
      nameTextStyle: { color: muted, fontSize: 11 },
      axisLabel: { color: muted, fontSize: 10 },
      splitLine: { lineStyle: { color: rule, type: 'dashed' } }
    },
    yAxis: {
      type: 'category',
      data: streamingData.map(function(d) { return d.name; }),
      axisLabel: { color: ink, fontSize: 11, fontWeight: 600 },
      axisLine: { lineStyle: { color: rule } },
      axisTick: { show: false }
    },
    series: [{
      type: 'bar',
      data: streamingData.map(function(d) {
        var colorMap = {
          '重叠': accent,
          '动量': accent2,
          '波动': green,
          '成交量': orange,
          '滤波器': '#6366f1'
        };
        return {
          value: d.thru,
          itemStyle: {
            color: colorMap[d.cat] || accent,
            borderRadius: [0, 4, 4, 0]
          }
        };
      }),
      barWidth: '50%',
      label: {
        show: true,
        position: 'right',
        formatter: function(p) { return p.value.toFixed(1) + ' G/s'; },
        color: muted,
        fontSize: 10
      }
    }],
    animation: false
  });
  window.addEventListener('resize', function() { chart6.resize(); });

})();