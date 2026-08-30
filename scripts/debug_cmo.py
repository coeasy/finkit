#!/usr/bin/env python3
"""
Debug CMO calculation differences between TA-Lib and AlphaTA
"""
import sys
import numpy as np

sys.path.insert(0, r'P:\llm_code\finkit\dist\python\windows-x64')

try:
    import finkit
    import talib
except ImportError as e:
    print(f"Import error: {e}")
    sys.exit(1)

# Generate test data
np.random.seed(42)
n = 10000
close = np.cumsum(np.random.randn(n)) + 100

print("=" * 80)
print("CMO Debug Analysis")
print("=" * 80)

# Calculate CMO
talib_cmo = talib.CMO(close, timeperiod=14)
alpha_cmo = np.array(finkit.cmo(close, timeperiod=14))

# Manual CMO calculation to understand the logic
changes = np.diff(close)
up = np.maximum(changes, 0)
down = np.maximum(-changes, 0)

print("\n[1] First valid CMO value (index=14)")
print("-" * 80)
print(f"TA-Lib CMO[14]: {talib_cmo[14]:.6f}")
print(f"AlphaTA CMO[14]: {alpha_cmo[14]:.6f}")

# Manual calculation for index 14
manual_up = np.sum(up[0:14])
manual_down = np.sum(down[0:14])
manual_cmo = 100 * (manual_up - manual_down) / (manual_up + manual_down)
print(f"\nManual calculation (window [0:14]):")
print(f"  sum_up: {manual_up:.6f}")
print(f"  sum_down: {manual_down:.6f}")
print(f"  CMO: {manual_cmo:.6f}")

# Try different window interpretations
manual_up2 = np.sum(up[1:15])
manual_down2 = np.sum(down[1:15])
manual_cmo2 = 100 * (manual_up2 - manual_down2) / (manual_up2 + manual_down2)
print(f"\nManual calculation (window [1:15]):")
print(f"  sum_up: {manual_up2:.6f}")
print(f"  sum_down: {manual_down2:.6f}")
print(f"  CMO: {manual_cmo2:.6f}")

print("\n[2] Compare values at different indices")
print("-" * 80)
for idx in [14, 50, 100, 150, 200]:
    print(f"\nIndex {idx}:")
    print(f"  TA-Lib:  {talib_cmo[idx]:.6f}")
    print(f"  AlphaTA: {alpha_cmo[idx]:.6f}")
    print(f"  Diff:    {abs(talib_cmo[idx] - alpha_cmo[idx]):.6f}")
    
    # Manual calculation
    if idx >= 14:
        m_up = np.sum(up[idx-14:idx])
        m_down = np.sum(down[idx-14:idx])
        m_cmo = 100 * (m_up - m_down) / (m_up + m_down)
        print(f"  Manual:  {m_cmo:.6f}")

print("\n[3] Check if AlphaTA uses different window size")
print("-" * 80)
# Try period=15 window
for idx in [14, 50, 100]:
    m_up15 = np.sum(up[idx-15:idx])
    m_down15 = np.sum(down[idx-15:idx])
    m_cmo15 = 100 * (m_up15 - m_down15) / (m_up15 + m_down15)
    print(f"Index {idx} with window=15: {m_cmo15:.6f} (AlphaTA: {alpha_cmo[idx]:.6f})")

print("\n" + "=" * 80)
