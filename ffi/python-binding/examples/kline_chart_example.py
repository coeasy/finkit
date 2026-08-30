import finkit as ta
import os
import math

def generate_kline_data(n=100):
    dates = []
    opens = []
    highs = []
    lows = []
    closes = []
    volumes = []
    price = 100.0
    for i in range(n):
        day = i + 1
        month = (day - 1) // 30 + 1
        d = (day - 1) % 30 + 1
        dates.append("2024-{:02d}-{:02d}".format(month, d))
        change = math.sin(i * 0.1) * 2.0 + (i % 7 - 3) * 0.5
        o = price
        c = price + change
        h = max(o, c) + abs(change) * 0.3
        l = min(o, c) - abs(change) * 0.3
        v = 1000.0 + math.sin(i * 0.2) * 500.0 + i * 10.0
        opens.append(round(o, 2))
        highs.append(round(h, 2))
        lows.append(round(l, 2))
        closes.append(round(c, 2))
        volumes.append(round(v, 2))
        price = c
    return ta.PyKlineData(
        dates=dates,
        opens=opens,
        highs=highs,
        lows=lows,
        closes=closes,
        volumes=volumes,
    )

def main():
    data = generate_kline_data(100)

    chart = ta.PyKlineChart(data, language="zh", title="K线图示例", width=1200, height=600)

    chart.add_ma([5, 10, 20])
    chart.add_macd(12, 26, 9)
    chart.add_rsi(14)

    output_dir = os.path.dirname(os.path.abspath(__file__))
    svg_path = os.path.join(output_dir, "kline_chart_example.svg")
    html_path = os.path.join(output_dir, "kline_chart_example.html")

    chart.save_as_svg(svg_path)
    print("SVG saved to:", svg_path)

    chart.save_as_html(html_path)
    print("HTML saved to:", html_path)

    svg_str = chart.to_svg_string()
    assert svg_str.startswith("<svg"), "SVG output should start with <svg"
    assert "</svg>" in svg_str, "SVG output should contain </svg>"
    print("SVG validation passed, length:", len(svg_str))

if __name__ == "__main__":
    main()
