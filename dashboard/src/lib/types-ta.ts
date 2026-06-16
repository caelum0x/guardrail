/**
 * Types for the `GET /ta` technical-analysis compute endpoint.
 *
 * The endpoint computes a single indicator over a caller-supplied close-price
 * series. Warmup positions come back as JSON `null` (NaN on the server), so all
 * value arrays are `(number | null)[]`.
 *
 * Mirrors the shapes produced by `apps/guardrail-api/src/ta.rs`.
 */

/** Indicators the endpoint supports over a close-price series. */
export type TaIndicator = "sma" | "ema" | "rsi" | "macd" | "bollinger";

/** All supported indicators, in display order. */
export const TA_INDICATORS: readonly TaIndicator[] = [
  "sma",
  "ema",
  "rsi",
  "macd",
  "bollinger",
] as const;

/** A single indicator value: a finite number, or `null` during warmup. */
export type TaValue = number | null;

/** Result body for the single-line indicators (`sma`, `ema`, `rsi`). */
export interface TaLineResult {
  values: TaValue[];
}

/** Result body for `macd` (three aligned series + the periods used). */
export interface TaMacdResult {
  macd: TaValue[];
  signal: TaValue[];
  histogram: TaValue[];
  params: {
    fast: number;
    slow: number;
    signal: number;
  };
}

/** Result body for `bollinger` (upper/middle/lower bands + multiplier). */
export interface TaBollingerResult {
  upper: TaValue[];
  middle: TaValue[];
  lower: TaValue[];
  mult: number;
}

/** Union of every possible `result` payload. */
export type TaResult = TaLineResult | TaMacdResult | TaBollingerResult;

/** Successful response envelope from `GET /ta`. */
export interface TaSuccessResponse {
  indicator: TaIndicator;
  period: number;
  input_len: number;
  result: TaResult;
  error?: undefined;
}

/** Error response envelope (bad series, period, or unknown indicator). */
export interface TaErrorResponse {
  error: string;
  supported?: string[];
  indicator?: undefined;
}

/** Either a successful or error response. */
export type TaResponse = TaSuccessResponse | TaErrorResponse;

/** A named series ready to render as a table column. */
export interface TaColumn {
  label: string;
  values: TaValue[];
}

/** Type guard: the response carries computed indicator data. */
export function isTaSuccess(
  response: TaResponse | null,
): response is TaSuccessResponse {
  return (
    response !== null &&
    typeof (response as TaSuccessResponse).indicator === "string" &&
    (response as TaErrorResponse).error === undefined
  );
}

/**
 * Flatten any `result` payload into one or more named columns. Each indicator
 * exposes a different set of series, so this normalises them for table render.
 */
export function toColumns(response: TaSuccessResponse): TaColumn[] {
  switch (response.indicator) {
    case "macd": {
      const r = response.result as TaMacdResult;
      return [
        { label: "MACD", values: r.macd },
        { label: "Signal", values: r.signal },
        { label: "Histogram", values: r.histogram },
      ];
    }
    case "bollinger": {
      const r = response.result as TaBollingerResult;
      return [
        { label: "Upper", values: r.upper },
        { label: "Middle", values: r.middle },
        { label: "Lower", values: r.lower },
      ];
    }
    default: {
      const r = response.result as TaLineResult;
      return [{ label: response.indicator.toUpperCase(), values: r.values }];
    }
  }
}
