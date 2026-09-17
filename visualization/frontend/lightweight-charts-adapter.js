/**
 * Finkit Lightweight Charts adapter.
 *
 * The Rust side produces the versioned payload; this module only owns the
 * browser chart instance. It supports both the current `addSeries` API and
 * the older named `add*Series` methods so applications can pin their own
 * Lightweight Charts release without changing the Finkit payload contract.
 */

function addSeries(chart, lightweightCharts, type, options) {
  if (typeof chart.addSeries === 'function' && lightweightCharts[type]) {
    return chart.addSeries(lightweightCharts[type], options || {});
  }
  const legacyMethod = {
    CandlestickSeries: 'addCandlestickSeries',
    HistogramSeries: 'addHistogramSeries',
    LineSeries: 'addLineSeries',
  }[type];
  if (!legacyMethod || typeof chart[legacyMethod] !== 'function') {
    throw new Error(`Unsupported Lightweight Charts series API: ${type}`);
  }
  return chart[legacyMethod](options || {});
}

function lineData(points) {
  // Lightweight Charts represents a missing point as whitespace `{time}`.
  return points.map((point) => point.value === null
    ? { time: point.time }
    : point);
}

/**
 * Create a chart from a `LightweightChartsPayload`.
 *
 * @param {HTMLElement|string} container Chart container or its DOM id.
 * @param {object} payload Rust-generated payload.
 * @param {object} lightweightCharts Imported Lightweight Charts namespace.
 * @param {object} [options] Optional chart and series options.
 * @returns {{chart: object, candle: object, volume: object, lines: Map<string, object>, setPayload: Function, update: Function}}
 */
export function createFinkitLightweightChart(container, payload, lightweightCharts, options = {}) {
  const element = typeof container === 'string'
    ? document.getElementById(container)
    : container;
  if (!element) throw new Error('Lightweight Charts container not found');
  if (!payload || payload.schema_version !== 1) {
    throw new Error('Unsupported Finkit Lightweight Charts payload schema');
  }

  const chart = lightweightCharts.createChart(element, options.chart || {});
  const candle = addSeries(chart, lightweightCharts, 'CandlestickSeries', options.candle);
  const volume = addSeries(chart, lightweightCharts, 'HistogramSeries', {
    priceFormat: { type: 'volume' },
    priceScaleId: '',
    ...(options.volume || {}),
  });
  const lines = new Map();

  function apply(next) {
    if (!next || next.schema_version !== 1) {
      throw new Error('Unsupported Finkit Lightweight Charts payload schema');
    }
    candle.setData(next.candles || []);
    volume.setData(next.volume || []);
    const incoming = new Set();
    for (const line of next.lines || []) {
      incoming.add(line.name);
      let series = lines.get(line.name);
      if (!series) {
        series = addSeries(chart, lightweightCharts, 'LineSeries', options.lines?.[line.name]);
        lines.set(line.name, series);
      }
      series.setData(lineData(line.data || []));
    }
    for (const [name, series] of lines) {
      if (!incoming.has(name) && typeof chart.removeSeries === 'function') {
        chart.removeSeries(series);
        lines.delete(name);
      }
    }
  }

  apply(payload);
  return {
    chart,
    candle,
    volume,
    lines,
    setPayload: apply,
    update(next) {
      if (!next || next.schema_version !== 1) {
        throw new Error('Unsupported Finkit Lightweight Charts payload schema');
      }
      if (next.candles?.length) candle.update(next.candles[next.candles.length - 1]);
      if (next.volume?.length) volume.update(next.volume[next.volume.length - 1]);
      for (const line of next.lines || []) {
        const series = lines.get(line.name);
        const point = line.data?.[line.data.length - 1];
        if (series && point) series.update(lineData([point])[0]);
      }
    },
  };
}

