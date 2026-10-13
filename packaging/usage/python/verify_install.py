import math

import finkit

result = finkit.sma([1.0, 2.0, 3.0, 4.0], 3)
assert len(result) == 4, result
assert math.isnan(float(result[0])) and math.isnan(float(result[1])), result
assert abs(float(result[2]) - 2.0) < 1e-12, result
assert abs(float(result[3]) - 3.0) < 1e-12, result
print("installed Python wheel SMA smoke passed")
