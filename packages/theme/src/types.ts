/** Semantic color roles. Every theme sets all of them; components use only these. */
export const COLOR_ROLES = [
  "background",
  "surface",
  "surfaceRaised",
  "border",
  "borderStrong",
  "text",
  "textMuted",
  "accent",
  "onAccent",
  "focus",
  "danger",
  "success",
  "warning",
  "selection",
] as const;

export type ColorRole = (typeof COLOR_ROLES)[number];

export const FONT_FAMILIES = ["sans", "serif", "mono"] as const;
export type FontFamily = (typeof FONT_FAMILIES)[number];

export const HEADING_CASES = ["none", "lowercase", "uppercase"] as const;
export type HeadingCase = (typeof HEADING_CASES)[number];

export const DENSITIES = ["compact", "comfortable", "spacious"] as const;
export type Density = (typeof DENSITIES)[number];

export const ELEVATIONS = ["flat", "raised"] as const;
export type Elevation = (typeof ELEVATIONS)[number];

export const MODES = ["light", "dark"] as const;
export type Mode = (typeof MODES)[number];

/** Version 1 of the theme format. `schema/theme.schema.json` describes the same shape. */
export interface Theme {
  schemaVersion: 1;
  id: string;
  name: string;
  description?: string;
  mode: Mode;
  colors: Record<ColorRole, string>;
  typography: {
    body: FontFamily;
    heading: FontFamily;
    /** Multiplies the whole type scale. 0.875 to 1.25. */
    scale: number;
    headingCase: HeadingCase;
  };
  /** Base corner radius in px, 0 to 16. */
  radius: number;
  density: Density;
  elevation: Elevation;
  /** `false` turns every transition and animation off, whatever the OS setting. */
  motion: boolean;
}
