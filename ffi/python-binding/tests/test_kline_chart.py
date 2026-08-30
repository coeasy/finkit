import pytest
import alpha_ta


class TestKlineData:
    def test_create_kline_data(self):
        data = alpha_ta.KlineData(
            dates=["2024-01-01", "2024-01-02"],
            opens=[100.0, 102.0],
            highs=[105.0, 106.0],
            lows=[98.0, 100.0],
            closes=[103.0, 104.0],
            volumes=[1000.0, 1200.0],
        )
        assert len(data) == 2

    def test_validate_valid_data(self):
        data = alpha_ta.KlineData(
            dates=["2024-01-01"],
            opens=[100.0],
            highs=[105.0],
            lows=[98.0],
            closes=[103.0],
            volumes=[1000.0],
        )
        assert data.validate() is True

    def test_validate_empty_data(self):
        data = alpha_ta.KlineData(
            dates=[], opens=[], highs=[], lows=[], closes=[], volumes=[]
        )
        assert data.validate() is False

    def test_push(self):
        data = alpha_ta.KlineData(
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
        data = alpha_ta.KlineData.from_json(json_str)
        assert len(data) == 1
        assert data.closes[0] == 103.0

    def test_from_csv(self):
        csv_str = "date,open,high,low,close,volume\n2024-01-01,100.0,105.0,98.0,103.0,1000.0\n2024-01-02,103.0,108.0,101.0,107.0,1200.0"
        data = alpha_ta.KlineData.from_csv(csv_str)
        assert len(data) == 2
        assert data.opens[0] == 100.0

    def test_getters(self):
        data = alpha_ta.KlineData(
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


class TestKlineChart:
    @pytest.fixture
    def sample_data(self):
        return alpha_ta.KlineData(
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
        chart = alpha_ta.KlineChart(sample_data)
        assert chart is not None

    def test_create_chart_with_params(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data, language="en", title="Test Chart", width=800, height=400)
        assert chart is not None

    def test_add_ma(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_ma([5, 10, 20])

    def test_add_ema(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_ema([12, 26])

    def test_add_boll(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_boll(period=20, nb_dev=2.0)

    def test_add_macd(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_macd(fast=12, slow=26, signal=9)

    def test_add_rsi(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_rsi(period=14)

    def test_add_kdj(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_kdj(fast_k=9, slow_k=3, slow_d=3)

    def test_add_sar(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        chart.add_sar(acceleration=0.02, maximum=0.2)

    def test_to_svg_string(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data)
        svg = chart.to_svg_string()
        assert svg.startswith("<svg")
        assert svg.endswith("</svg>")

    def test_to_svg_string_with_indicators(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data, title="Test K-Line")
        chart.add_ma([5, 10])
        chart.add_boll()
        svg = chart.to_svg_string()
        assert svg.startswith("<svg")
        assert "Test K-Line" in svg

    def test_save_as_svg(self, sample_data, tmp_path):
        chart = alpha_ta.KlineChart(sample_data)
        svg_path = str(tmp_path / "test_chart.svg")
        chart.save_as_svg(svg_path)
        with open(svg_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert content.startswith("<svg")

    def test_save_as_html(self, sample_data, tmp_path):
        chart = alpha_ta.KlineChart(sample_data)
        html_path = str(tmp_path / "test_chart.html")
        chart.save_as_html(html_path)
        with open(html_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert "<!DOCTYPE html>" in content
        assert "<svg" in content

    def test_chart_with_all_indicators(self, sample_data):
        chart = alpha_ta.KlineChart(sample_data, language="zh", title="Full Chart")
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
        data = alpha_ta.KlineData(
            dates=[], opens=[], highs=[], lows=[], closes=[], volumes=[]
        )
        chart = alpha_ta.KlineChart(data)
        with pytest.raises(ValueError):
            chart.to_svg_string()
