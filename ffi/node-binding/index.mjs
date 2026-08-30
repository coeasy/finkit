// ESM entry point
import { createRequire } from 'module'
import { platform, arch } from 'process'
import { readFileSync } from 'fs'
import { join, dirname } from 'path'
import { fileURLToPath } from 'url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const require = createRequire(import.meta.url)

let nativeBinding = null

try {
  nativeBinding = require('./finkit.node')
} catch (e) {
  let pkgName = null
  const platformKey = `${platform}-${arch}`

  switch (platformKey) {
    case 'darwin-arm64':
      pkgName = 'finkit-darwin-arm64'
      break
    case 'darwin-x64':
      pkgName = 'finkit-darwin-x64'
      break
    case 'linux-arm64':
      try {
        if (readFileSync('/usr/bin/ldd', 'utf8').includes('musl')) {
          pkgName = 'finkit-linux-arm64-musl'
        } else {
          pkgName = 'finkit-linux-arm64-gnu'
        }
      } catch {
        pkgName = 'finkit-linux-arm64-gnu'
      }
      break
    case 'linux-x64':
      try {
        if (readFileSync('/usr/bin/ldd', 'utf8').includes('musl')) {
          pkgName = 'finkit-linux-x64-musl'
        } else {
          pkgName = 'finkit-linux-x64-gnu'
        }
      } catch {
        pkgName = 'finkit-linux-x64-gnu'
      }
      break
    case 'win32-x64':
      pkgName = 'finkit-win32-x64-msvc'
      break
    case 'win32-arm64':
      pkgName = 'finkit-win32-arm64-msvc'
      break
    default:
      throw new Error(`Unsupported platform: ${platformKey}`)
  }

  if (pkgName) {
    try {
      nativeBinding = await import(pkgName)
    } catch (err) {
      throw new Error(
        `Failed to load native binding for ${platformKey}. ` +
        `Try installing ${pkgName} manually. ` +
        `Original error: ${err.message}`
      )
    }
  }
}

if (!nativeBinding) {
  throw new Error('Failed to load native Rust TA-Lib binding')
}

export const sma = nativeBinding.sma
export const ema = nativeBinding.ema
export const wma = nativeBinding.wma
export const dema = nativeBinding.dema
export const tema = nativeBinding.tema
export const kama = nativeBinding.kama
export const mama = nativeBinding.mama
export const t3 = nativeBinding.t3
export const rsi = nativeBinding.rsi
export const macd = nativeBinding.macd
export const macdAsync = nativeBinding.macd_async
export const stoch = nativeBinding.stoch
export const adx = nativeBinding.adx
export const atr = nativeBinding.atr
export const obv = nativeBinding.obv
export const sar = nativeBinding.sar
export const aroon = nativeBinding.aroon
export const cci = nativeBinding.cci
export const mom = nativeBinding.mom
export const roc = nativeBinding.roc
export const willr = nativeBinding.willr
export const apo = nativeBinding.apo
export const bop = nativeBinding.bop
export const cmo = nativeBinding.cmo
export const dx = nativeBinding.dx
export const mfi = nativeBinding.mfi
export const minusDi = nativeBinding.minus_di
export const plusDi = nativeBinding.plus_di
export const trix = nativeBinding.trix
export const ad = nativeBinding.ad
export const adosc = nativeBinding.adosc
export const bollingerBands = nativeBinding.bollinger_bands
export const natr = nativeBinding.natr
export const trange = nativeBinding.trange
export const htDcperiod = nativeBinding.ht_dcperiod
export const htDcphase = nativeBinding.ht_dcphase
export const htPhasor = nativeBinding.ht_phasor
export const htSine = nativeBinding.ht_sine
export const htTrendmode = nativeBinding.ht_trendmode
export const htTrendline = nativeBinding.ht_trendline
export const avgprice = nativeBinding.avgprice
export const medprice = nativeBinding.medprice
export const typprice = nativeBinding.typprice
export const wclprice = nativeBinding.wclprice
export const zscore = nativeBinding.zscore
export const percentRank = nativeBinding.percent_rank
export const beta = nativeBinding.beta
export const correlation = nativeBinding.correlation
export const stdDev = nativeBinding.std_dev
export const linearReg = nativeBinding.linear_reg
export const tsf = nativeBinding.tsf
export const cdlDoji = nativeBinding.cdl_doji
export const cdlDragonflyDoji = nativeBinding.cdl_dragonfly_doji
export const cdlGravestoneDoji = nativeBinding.cdl_gravestone_doji
export const cdlLongLeggedDoji = nativeBinding.cdl_long_legged_doji
export const cdlHammer = nativeBinding.cdl_hammer
export const cdlInvertedHammer = nativeBinding.cdl_inverted_hammer
export const cdlHangingMan = nativeBinding.cdl_hanging_man
export const cdlShootingStar = nativeBinding.cdl_shooting_star
export const cdlEngulfing = nativeBinding.cdl_engulfing
export const cdlHarami = nativeBinding.cdl_harami
export const cdlHaramiCross = nativeBinding.cdl_harami_cross
export const cdlMorningStar = nativeBinding.cdl_morning_star
export const cdlEveningStar = nativeBinding.cdl_evening_star
export const cdlMorningDojiStar = nativeBinding.cdl_morning_doji_star
export const cdlEveningDojiStar = nativeBinding.cdl_evening_doji_star
export const cdlThreeWhiteSoldiers = nativeBinding.cdl_three_white_soldiers
export const cdlThreeBlackCrows = nativeBinding.cdl_three_black_crows
export const cdlMarubozu = nativeBinding.cdl_marubozu
export const cdlPiercing = nativeBinding.cdl_piercing
export const cdlDarkCloudCover = nativeBinding.cdl_dark_cloud_cover
export const cdlBeltHold = nativeBinding.cdl_belt_hold
export const cdlSpinningTop = nativeBinding.cdl_spinning_top
export const cdlHighWave = nativeBinding.cdl_high_wave
export const cdlRickshawMan = nativeBinding.cdl_rickshaw_man
export const cdlTweezerTop = nativeBinding.cdl_tweezer_top
export const cdlTweezerBot = nativeBinding.cdl_tweezer_bot
export const cdlKicking = nativeBinding.cdl_kicking
export const detectHeadShoulders = nativeBinding.detect_head_shoulders
export const detectDoubleTop = nativeBinding.detect_double_top
export const detectDoubleBottom = nativeBinding.detect_double_bottom
export const detectHeadShouldersBottom = nativeBinding.detect_head_shoulders_bottom
export const detectTripleTop = nativeBinding.detect_triple_top
export const detectTripleBottom = nativeBinding.detect_triple_bottom
