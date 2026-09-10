# -*- coding: utf-8 -*-
"""和风天气工具模块

基于 QWeather API v7，提供：
- 城市搜索（中文名 → LocationID）
- 实时天气查询
- 3 天天气预报

不依赖 LangChain / requests，使用纯 urllib（与项目风格一致）。
"""

import gzip
import json
import os
import sys
from typing import Optional
from urllib.parse import quote
from urllib.request import urlopen, Request
from urllib.error import URLError

from dotenv import load_dotenv

try:
    from loguru import logger
except ImportError:  # loguru 未安装时退化为标准 stderr 输出
    class _Logger:
        @staticmethod
        def error(msg): print(msg, file=sys.stderr)
        @staticmethod
        def info(msg): print(msg, file=sys.stderr)
        @staticmethod
        def warning(msg): print(msg, file=sys.stderr)

    logger = _Logger()

# 确保 .env 已加载
load_dotenv()

# ── API 配置 ──────────────────────────────────────────────────

GEO_API_URL = "https://mw3aapta8r.re.qweatherapi.com/geo/v2/city/lookup"
WEATHER_API_URL = "https://mw3aapta8r.re.qweatherapi.com/v7/weather"
API_KEY = os.getenv("X-QW-Api-Key", "")


# ── 天气数据类 ────────────────────────────────────────────────

class CurrentWeather:
    """实时天气"""
    def __init__(self, data: dict) -> None:
        self.temp: str = data.get("temp", "")
        self.feels_like: str = data.get("feelsLike", "")
        self.text: str = data.get("text", "")
        self.wind_dir: str = data.get("windDir", "")
        self.wind_scale: str = data.get("windScale", "")
        self.wind_speed: str = data.get("windSpeed", "")
        self.humidity: str = data.get("humidity", "")
        self.precip: str = data.get("precip", "")
        self.pressure: str = data.get("pressure", "")
        self.vis: str = data.get("vis", "")
        self.cloud: str = data.get("cloud", "")
        self.obs_time: str = data.get("obsTime", "")


class DayForecast:
    """单日天气预报"""
    def __init__(self, data: dict) -> None:
        self.fx_date: str = data.get("fxDate", "")
        self.sunrise: str = data.get("sunrise", "")
        self.sunset: str = data.get("sunset", "")
        self.temp_max: str = data.get("tempMax", "")
        self.temp_min: str = data.get("tempMin", "")
        self.text_day: str = data.get("textDay", "")
        self.text_night: str = data.get("textNight", "")
        self.wind_dir_day: str = data.get("windDirDay", "")
        self.wind_scale_day: str = data.get("windScaleDay", "")
        self.wind_dir_night: str = data.get("windDirNight", "")
        self.wind_scale_night: str = data.get("windScaleNight", "")
        self.humidity: str = data.get("humidity", "")
        self.precip: str = data.get("precip", "")
        self.pressure: str = data.get("pressure", "")
        self.uv_index: str = data.get("uvIndex", "")
        self.vis: str = data.get("vis", "")


class WeatherResult:
    """完整天气查询结果"""
    def __init__(
        self,
        city_name: str = "",
        adm1: str = "",
        adm2: str = "",
        now: Optional[CurrentWeather] = None,
        forecasts: Optional[list[DayForecast]] = None,
    ) -> None:
        self.city_name = city_name
        self.adm1 = adm1          # 省份
        self.adm2 = adm2          # 城市
        self.now = now
        self.forecasts = forecasts or []

    def format_for_llm(self) -> str:
        """格式化为 LLM 可理解的结构化文本"""
        lines = [f"城市：{self.adm1} {self.adm2}（{self.city_name}）"]

        if self.now:
            lines.append(
                f"实时天气：{self.now.text}，温度 {self.now.temp}℃，"
                f"体感 {self.now.feels_like}℃，湿度 {self.now.humidity}%，"
                f"{self.now.wind_dir} {self.now.wind_scale}级"
            )

        if self.forecasts:
            lines.append(f"\n未来 {len(self.forecasts)} 天预报：")
            for i, fc in enumerate(self.forecasts):
                day_label = ["今天", "明天", "后天"][i] if i < 3 else f"第{i+1}天"
                lines.append(
                    f"  {day_label}（{fc.fx_date}）："
                    f"白天 {fc.text_day}，夜间 {fc.text_night}，"
                    f"{fc.temp_min}℃ ~ {fc.temp_max}℃，"
                    f"白天{fc.wind_dir_day} {fc.wind_scale_day}级，"
                    f"湿度 {fc.humidity}%，紫外线 {fc.uv_index}"
                )

        return "\n".join(lines)

    def format_brief(self) -> str:
        """简短格式（用于弹窗）"""
        lines = []
        if self.forecasts and len(self.forecasts) >= 2:
            today = self.forecasts[0]
            tomorrow = self.forecasts[1]
            lines.append(
                f"☀ 今天：{today.text_day}，{today.temp_min}~{today.temp_max}℃"
            )
            lines.append(
                f"🌙 明天：{tomorrow.text_day}，{tomorrow.temp_min}~{tomorrow.temp_max}℃"
            )
        if self.now:
            lines.append(
                f"🌡 当前：{self.now.temp}℃（体感 {self.now.feels_like}℃）"
                f"  {self.now.wind_dir} {self.now.wind_scale}级"
            )
        return "\n".join(lines)


# ── API 请求辅助 ──────────────────────────────────────────────

def _make_request(url: str, timeout: int = 10) -> Optional[dict]:
    """发送 GET 请求，返回解析后的 JSON"""
    if not API_KEY:
        logger.error("[Weather] API Key 未配置（X-QW-Api-Key）")
        return None

    # 将 API Key 作为 URL 参数
    separator = "&" if "?" in url else "?"
    url_with_key = f"{url}{separator}key={API_KEY}"

    try:
        req = Request(url_with_key)
        with urlopen(req, timeout=timeout) as resp:
            raw = resp.read()
            # 处理 gzip 压缩
            if raw[:2] == b"\x1f\x8b":
                raw = gzip.decompress(raw)
            data = json.loads(raw.decode("utf-8"))
        if data.get("code") != "200":
            logger.error(f"[Weather] API 错误: code={data.get('code')}")
            return None
        return data
    except URLError as e:
        logger.error(f"[Weather] 网络错误: {e}")
        return None
    except json.JSONDecodeError as e:
        logger.error(f"[Weather] JSON 解析失败: {e}")
        return None


# ── 天气工具函数 ──────────────────────────────────────────────

def lookup_city(city_name: str) -> Optional[dict]:
    """城市搜索

    :param city_name: 中文城市名（如"西安"、"北京"）
    :return: {"id": "101110101", "name": "西安", "adm1": "陕西省", "adm2": "西安市"}
    """
    url = f"{GEO_API_URL}?location={quote(city_name)}&number=1"
    data = _make_request(url)
    if data and data.get("location"):
        loc = data["location"][0]
        result = {
            "id": loc["id"],
            "name": loc.get("name", city_name),
            "adm1": loc.get("adm1", ""),
            "adm2": loc.get("adm2", ""),
            "lat": loc.get("lat", ""),
            "lon": loc.get("lon", ""),
        }
        logger.info(f"[Weather] 城市查询: {city_name} → {result['id']}")
        return result
    logger.warning(f"[Weather] 城市查询失败: {city_name}")
    return None


def get_weather_now(location_id: str) -> Optional[CurrentWeather]:
    """获取实时天气

    :param location_id: 城市 LocationID
    """
    url = f"{WEATHER_API_URL}/now?location={location_id}"
    data = _make_request(url)
    if data and data.get("now"):
        return CurrentWeather(data["now"])
    return None


def get_weather_3d(location_id: str) -> Optional[list[DayForecast]]:
    """获取 3 天天气预报

    :param location_id: 城市 LocationID
    :return: 3 天的 DayForecast 列表 [今天, 明天, 后天]
    """
    url = f"{WEATHER_API_URL}/3d?location={location_id}"
    data = _make_request(url)
    if data and data.get("daily"):
        return [DayForecast(d) for d in data["daily"]]
    return None


def get_full_weather(city_name: str) -> Optional[WeatherResult]:
    """便捷函数：一次性获取某个城市的完整天气信息

    :param city_name: 中文城市名
    :return: WeatherResult（含实时天气 + 3天预报）
    """
    loc = lookup_city(city_name)
    if not loc:
        return None

    now = get_weather_now(loc["id"])
    forecasts = get_weather_3d(loc["id"])

    return WeatherResult(
        city_name=loc["name"],
        adm1=loc["adm1"],
        adm2=loc["adm2"],
        now=now,
        forecasts=forecasts,
    )


def get_default_weather() -> Optional[WeatherResult]:
    """获取默认城市（西安）的天气"""
    return get_full_weather("西安")


def main() -> int:
    """CLI 入口：python weather_tool.py <城市名>"""
    if len(sys.argv) < 2:
        print("用法: python weather_tool.py <城市名>", file=sys.stderr)
        return 2

    city = sys.argv[1]
    result = get_full_weather(city)

    if result is None:
        print(f"未查询到「{city}」的天气信息", file=sys.stderr)
        return 1

    print(result.format_for_llm())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
