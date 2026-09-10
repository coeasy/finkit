import pytest
import numpy as np
import finkit
from datetime import datetime, timezone


class TestKlineData:
    def test_create_kline_data(self):
        data = finkit.KlineData(
            dates=["2024-01-01", "2024-01-02"],
            opens=[100.0, 102.0],
            highs=[105.0, 106.0],
            lows=[98.0, 100.0],
            closes=[103.0, 104.0],
            volumes=[1000.0, 1200.0],
        )
        assert len(data) == 2

    def test_validate_valid_data(self):
        data = finkit.KlineData(
            dates=["2024-01-01"],
            opens=[100.0],
            highs=[105.0],
            lows=[98.0],
            closes=[103.0],
            volumes=[1000.0],
        )
        assert data.validate() is True
        assert data.validate_ohlcv() is True
        assert data.validation_errors() == []

    def test_validate_empty_data(self):
        data = finkit.KlineData(
            dates=[], opens=[], highs=[], lows=[], closes=[], volumes=[]
        )
        assert data.validate() is False
        assert data.validate_ohlcv() is False
        assert data.validation_errors()

    def test_push(self):
        data = finkit.KlineData(
            dates=["2024-01-01"],
            opens=[100.0],
            highs=[105.0],
            lows=[98.0],
            closes=[103.0],
            volumes=[1000.0],
        )
        data.push("2024-01-02", 103.0, 108.0, 101.0, 107.0, 1200.0)
        assert len(data) == 2

    def test_from_json(self):
        json_str = '{"dates":["2024-01-01"],"opens":[100.0],"highs":[105.0],"lows":[98.0],"closes":[103.0],"volumes":[1000.0]}'
        data = finkit.KlineData.from_json(json_str)
        assert len(data) == 1
        assert data.closes[0] == 103.0

    def test_from_csv(self):
        csv_str = "date,open,high,low,close,volume\n2024-01-01,100.0,105.0,98.0,103.0,1000.0\n2024-01-02,103.0,108.0,101.0,107.0,1200.0"
        data = finkit.KlineData.from_csv(csv_str)
        assert len(data) == 2
        assert data.opens[0] == 100.0

    def test_getters(self):
        data = finkit.KlineData(
            dates=["2024-01-01", "2024-01-02"],
            opens=[100.0, 102.0],
            highs=[105.0, 106.0],
            lows=[98.0, 100.0],
            closes=[103.0, 104.0],
            volumes=[1000.0, 1200.0],
        )
        assert data.dates == ["2024-01-01", "2024-01-02"]
        assert data.opens == [100.0, 102.0]
        assert data.highs == [105.0, 106.0]
        assert data.lows == [98.0, 100.0]
        assert data.closes == [103.0, 104.0]
        assert data.volumes == [1000.0, 1200.0]

    def test_timestamps_roundtrip(self):
        data = finkit.KlineData(
            dates=["2024-01-01", "2024-01-02"],
            opens=[100.0, 102.0],
            highs=[105.0, 106.0],
            lows=[98.0, 100.0],
            closes=[103.0, 104.0],
            volumes=[1000.0, 1200.0],
            timestamps=[1704067200, 1704153600],
        )
        assert data.timestamps == [1704067200, 1704153600]
        data.set_timestamps([1704067200, 1704153600])
        with pytest.raises(ValueError):
            data.set_timestamps([2, 1])

    def test_versioned_calendar_config_resolver(self):
        session = finkit.resolve_market_session_config(
            '{"market":"a_share","timezone":"Asia/Shanghai",'
            '"holidays":["2026-01-01"],"source":"sse-official",'
            '"revision":"2026.1"}',
            1767317400,  # 2026-01-02 09:30 Asia/Shanghai
        )
        assert session["source"] == "sse-official"
        assert session["revision"] == "2026.1"

        annual = finkit.resolve_market_session_csv(
            "date,status,sessions\n2026-01-01,closed,\n"
            "2026-01-02,open,09:30-11:30;13:00-15:00\n",
            "a_share",
            1767317400,
            "Asia/Shanghai",
        )
        assert annual["session_index"] == 0

    def test_builtin_market_calendar_resolver_supports_all_markets(self):
        timestamps = {
            "a_share": int(datetime(2024, 1, 5, 1, 30, tzinfo=timezone.utc).timestamp()),
            "china_futures": int(datetime(2024, 1, 5, 13, 0, tzinfo=timezone.utc).timestamp()),
            "hong_kong": int(datetime(2024, 1, 5, 1, 30, tzinfo=timezone.utc).timestamp()),
            "us_equity": int(datetime(2024, 1, 5, 14, 30, tzinfo=timezone.utc).timestamp()),
            "crypto": int(datetime(2024, 1, 7, 12, 0, tzinfo=timezone.utc).timestamp()),
        }
        for market, timestamp in timestamps.items():
            session = finkit.resolve_market_session(market, timestamp)
            assert session is not None
            assert session["market"] == market
            assert session["close_timestamp"] > session["open_timestamp"]

        assert finkit.resolve_market_session(
            "a_share",
            timestamps["a_share"],
            holidays=["2024-01-05"],
        ) is None

        custom = finkit.resolve_market_session(
            "a_share",
            timestamps["a_share"],
            sessions=[(9 * 3600 + 30 * 60, 10 * 3600)],
        )
        assert custom["session_index"] == 0


class TestKlineChart:
    @pytest.fixture
    def sample_data(self):
        return finkit.KlineData(
            dates=[
                "2024-01-02", "2024-01-03", "2024-01-04", "2024-01-05", "2024-01-08",
                "2024-01-09", "2024-01-10", "2024-01-11", "2024-01-12", "2024-01-15",
            ],
            opens=[100.0, 102.0, 101.0, 103.0, 105.0, 104.0, 106.0, 108.0, 107.0, 109.0],
            highs=[105.0, 106.0, 104.0, 107.0, 108.0, 107.0, 109.0, 110.0, 109.0, 111.0],
            lows=[98.0, 100.0, 99.0, 101.0, 103.0, 102.0, 104.0, 106.0, 105.0, 107.0],
            closes=[103.0, 104.0, 100.0, 105.0, 107.0, 103.0, 108.0, 106.0, 108.0, 110.0],
            volumes=[1000.0, 1200.0, 800.0, 1500.0, 2000.0, 1100.0, 1800.0, 900.0, 1300.0, 1600.0],
        )

    def test_create_chart_default(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        assert chart is not None

    def test_create_chart_with_params(self, sample_data):
        chart = finkit.KlineChart(sample_data, language="en", title="Test Chart", width=800, height=400)
        assert chart is not None

    def test_webgl_html_backend(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_ma([3])
        chart.add_event_marker(3, "突破候选", value=sample_data.highs[3])
        html = chart.to_webgl_html()
        assert 'data-renderer="webgl2"' in html
        assert "if(false){webgpuDraw=await tryWebGpu()}" in html
        assert "drawArraysInstanced" in html
        assert "navigator.gpu" in html
        assert "indicatorRows" in html
        assert "MA3" in html
        assert "突破候选" in html
        assert "enhancedTooltip" in html
        webgpu_html = chart.to_webgpu_html()
        assert "if(true){webgpuDraw=await tryWebGpu()}" in webgpu_html
        assert "@vertex fn vsBody" in webgpu_html
        assert "decodeF32" in webgpu_html
        assert "setRingBuffer:setRingBuffer" in webgpu_html
        assert "device.lost" in webgpu_html
        assert "gpuRecovery" in webgpu_html

    def test_add_ma(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_ma([5, 10, 20])

    def test_add_ema(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_ema([12, 26])

    def test_add_boll(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_boll(period=20, nb_dev=2.0)

    def test_add_macd(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_macd(fast=12, slow=26, signal=9)

    def test_add_rsi(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_rsi(period=14)

    def test_add_kdj(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_kdj(fast_k=9, slow_k=3, slow_d=3)

    def test_add_sar(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_sar(acceleration=0.02, maximum=0.2)

    def test_add_custom_indicator_and_interaction(self, sample_data, tmp_path):
        chart = finkit.KlineChart(sample_data)
        chart.add_custom_indicator("自定义线", [float(index) for index in range(len(sample_data))])
        chart.add_event_marker(3, "突破", value=sample_data.highs[3])
        chart.set_interaction(show_data_window=True, enable_pan_zoom=True)
        html_path = str(tmp_path / "custom_indicator.html")
        chart.save_as_html(html_path)
        with open(html_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert "自定义线" in content

    def test_custom_indicator_requires_source_alignment(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        with pytest.raises(ValueError):
            chart.add_custom_indicator("bad", [1.0])

    def test_custom_indicator_series_can_be_replaced(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_custom_indicator("signal", [1.0] * len(sample_data))
        chart.set_custom_indicator_series("signal", [2.0] * len(sample_data))
        html = chart.to_canvas_html()
        assert "signal" in html
        with pytest.raises(ValueError):
            chart.set_custom_indicator_series("signal", [1.0])

    def test_canvas_output(self, sample_data, tmp_path):
        chart = finkit.KlineChart(sample_data, title="Canvas")
        html = chart.to_canvas_html()
        assert 'data-renderer="canvas2d"' in html
        assert "getContext('2d')" in html
        path = tmp_path / "chart-canvas.html"
        chart.save_as_canvas_html(str(path))
        assert path.read_text(encoding="utf-8").startswith("<!DOCTYPE html>")

    def test_live_upsert_validates_and_revises_current_bar(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        assert chart.upsert_kline("2024-01-15", 109.0, 112.0, 106.0, 111.0, 1700.0) == "updated"
        assert chart.upsert_kline("2024-01-16", 111.0, 113.0, 110.0, 112.0, 1800.0) == "appended"
        assert chart.upsert_klines([
            ("2024-01-16", 111.0, 114.0, 110.0, 113.0, 1900.0),
            ("2024-01-17", 113.0, 115.0, 112.0, 114.0, 2000.0),
        ]) == ["updated", "appended"]
        with pytest.raises(ValueError):
            chart.append_kline("2024-01-17", 110.0, 109.0, 108.0, 112.0, 1800.0)

    def test_replay_window_and_next(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        assert chart.set_replay_window(window=4, cursor=4) == (1, 5)
        assert chart.replay_next() == (2, 6)

    def test_add_chan(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        chart.add_chan(min_stroke_bars=2, show_labels=True)
        chart.set_chan_thresholds(0.001, 0.002, 0.1, 0.001)
        svg = chart.to_svg_string()
        assert svg.startswith("<svg")
        chart.add_chan_multi([2, 4], variant="standard")
        assert chart.to_svg_string().startswith("<svg")

    def test_chan_analyze(self, sample_data):
        result = finkit.chan_analyze(
            np.asarray(sample_data.opens, dtype=np.float64),
            np.asarray(sample_data.highs, dtype=np.float64),
            np.asarray(sample_data.lows, dtype=np.float64),
            np.asarray(sample_data.closes, dtype=np.float64),
            np.asarray(sample_data.volumes, dtype=np.float64),
            min_stroke_bars=2,
        )
        assert result["bar_count"] > 0
        assert "fractals" in result
        assert "strokes" in result
        assert "trend" in result
        assert "signals" in result

    def test_chan_analyze_multi_auto_and_explicit(self, sample_data):
        close = np.asarray(sample_data.closes * 15, dtype=np.float64)
        opens = close - 0.2
        highs = close + 0.5
        lows = close - 0.5
        volumes = np.ones_like(close)
        result = finkit.chan_analyze_multi(
            opens,
            highs,
            lows,
            close,
            volumes,
            factors=[1, 5],
            variant="aggressive",
        )
        assert [frame["factor"] for frame in result["frames"]] == [1, 5]
        assert "analysis" in result["frames"][1]

    def test_chan_analyze_multi_timestamps_preserves_ranges(self, sample_data):
        close = np.asarray(sample_data.closes * 4, dtype=np.float64)
        opens = close - 0.2
        highs = close + 0.5
        lows = close - 0.5
        volumes = np.ones_like(close)
        timestamps = np.arange(close.size, dtype=np.int64) * 60
        result = finkit.chan_analyze_multi_timestamps(
            timestamps,
            opens,
            highs,
            lows,
            close,
            volumes,
            durations_seconds=[300],
            origin_seconds=0,
        )
        assert result["frames"][0]["label"] == "base"
        assert result["frames"][1]["seconds"] == 300
        assert len(result["frames"][1]["source_ranges"]) > 0

    def test_chan_analyze_multi_timestamps_calendar_market_timezone(self):
        close = np.asarray([100.0, 101.0, 99.0, 102.0, 103.0, 101.0, 104.0], dtype=np.float64)
        opens = close - 0.2
        highs = close + 0.5
        lows = close - 0.5
        volumes = np.ones_like(close)
        # 2024-01-05 09:30 Asia/Shanghai, plus a Saturday row that must be removed.
        friday_open = np.int64(1704418200)
        timestamps = np.asarray(
            [
                friday_open,
                friday_open + 60,
                friday_open + 3_600,
                friday_open + 4 * 3_600,
                friday_open + 4 * 3_600 + 60,
                friday_open + 86_400,
                friday_open + 3 * 86_400,
            ],
            dtype=np.int64,
        )
        result = finkit.chan_analyze_multi_timestamps_calendar(
            timestamps,
            opens,
            highs,
            lows,
            close,
            volumes,
            market="a_share",
            durations_seconds=[3_600],
        )
        assert result["frames"][0]["label"] == "base"
        assert result["frames"][0]["source_ranges"][-1] == (6, 7)
        assert result["frames"][1]["label"] == "3600s-calendar"

    def test_compute_composite_graph(self, sample_data):
        close = np.asarray(sample_data.closes + list(range(111, 141)), dtype=np.float64)
        result = finkit.compute_composite(
            close=close,
            definitions=[
                ("fast", "ema", ["close"], [5.0]),
                ("slow", "ema", ["close"], [10.0]),
                ("spread", "sub", ["fast", "slow"], []),
                ("signal", "sma", ["spread"], [3.0]),
            ],
            outputs=["signal"],
        )
        assert set(result.keys()) == {"signal"}
        assert len(result["signal"]) == len(close)

    def test_compute_composite_weighted_average(self):
        result = finkit.compute_composite(
            close=np.asarray([10.0, 20.0], dtype=np.float64),
            definitions=[
                ("weighted", "weighted_average", ["close", "const:20"], [1.0, 3.0]),
            ],
            outputs=["weighted"],
        )
        np.testing.assert_allclose(result["weighted"], [17.5, 20.0])

    def test_float64_indicators_return_numpy_arrays_directly(self, sample_data):
        close = np.asarray(sample_data.closes, dtype=np.float64)
        sma = finkit.sma(close, timeperiod=3)
        macd = finkit.macd(close, fastperiod=2, slowperiod=3, signalperiod=2)
        assert isinstance(sma, np.ndarray)
        assert sma.dtype == np.float64
        assert all(isinstance(item, np.ndarray) for item in macd)
        assert all(item.dtype == np.float64 for item in macd)

    def test_to_svg_string(self, sample_data):
        chart = finkit.KlineChart(sample_data)
        svg = chart.to_svg_string()
        assert svg.startswith("<svg")
        assert svg.endswith("</svg>")

    def test_to_svg_string_with_indicators(self, sample_data):
        chart = finkit.KlineChart(sample_data, title="Test K-Line")
        chart.add_ma([5, 10])
        chart.add_boll()
        svg = chart.to_svg_string()
        assert svg.startswith("<svg")
        assert "Test K-Line" in svg

    def test_save_as_svg(self, sample_data, tmp_path):
        chart = finkit.KlineChart(sample_data)
        svg_path = str(tmp_path / "test_chart.svg")
        chart.save_as_svg(svg_path)
        with open(svg_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert content.startswith("<svg")

    def test_save_as_html(self, sample_data, tmp_path):
        chart = finkit.KlineChart(sample_data)
        html_path = str(tmp_path / "test_chart.html")
        chart.save_as_html(html_path)
        with open(html_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert "<!DOCTYPE html>" in content
        assert "<svg" in content

    def test_chart_with_all_indicators(self, sample_data):
        chart = finkit.KlineChart(sample_data, language="zh", title="Full Chart")
        chart.add_ma([5, 10, 20])
        chart.add_ema([12, 26])
        chart.add_boll()
        chart.add_macd()
        chart.add_rsi()
        chart.add_kdj()
        chart.add_sar()
        svg = chart.to_svg_string()
        assert svg.startswith("<svg")

    def test_chart_empty_data_error(self):
        data = finkit.KlineData(
            dates=[], opens=[], highs=[], lows=[], closes=[], volumes=[]
        )
        chart = finkit.KlineChart(data)
        with pytest.raises(ValueError):
            chart.to_svg_string()
