import type { ColorRole, Theme } from "./types.ts";

/** WCAG 2.x relative luminance of a `#rrggbb` color. */
export function luminance(hex: string): number {
  const channel = (offset: number): number => {
    const value = Number.parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** WCAG 2.x contrast ratio between two `#rrggbb` colors, from 1 to 21. */
export function contrastRatio(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (light + 0.05) / (dark + 0.05);
}

export interface ContrastRequirement {
  foreground: ColorRole;
  background: ColorRole;
  /** 4.5 for text (WCAG 1.4.3), 3 for focus rings and control boundaries (WCAG 1.4.11). */
  minimum: number;
}

const SURFACES = ["background", "surface", "surfaceRaised"] as const;
const PAGE = ["background", "surface"] as const;

const pairs = (
  foreground: ColorRole,
  backgrounds: readonly ColorRole[],
  minimum: number,
): ContrastRequirement[] => backgrounds.map((background) => ({ foreground, background, minimum }));

/** Every pairing the components use, with the minimum each one needs for WCAG AA. */
export const CONTRAST_REQUIREMENTS: readonly ContrastRequirement[] = [
  ...pairs("text", SURFACES, 4.5),
  ...pairs("textMuted", SURFACES, 4.5),
  ...pairs("accent", SURFACES, 4.5),
  ...pairs("danger", PAGE, 4.5),
  ...pairs("success", PAGE, 4.5),
  ...pairs("warning", PAGE, 4.5),
  { foreground: "onAccent", background: "accent", minimum: 4.5 },
  { foreground: "text", background: "selection", minimum: 4.5 },
  ...pairs("borderStrong", SURFACES, 3),
  ...pairs("focus", SURFACES, 3),
];

export interface ContrastResult extends ContrastRequirement {
  ratio: number;
  passes: boolean;
}

/** Measures every requirement for a theme. */
export function measureContrast(theme: Theme): ContrastResult[] {
  return CONTRAST_REQUIREMENTS.map((requirement) => {
    const ratio = contrastRatio(
      theme.colors[requirement.foreground],
      theme.colors[requirement.background],
    );
    return { ...requirement, ratio, passes: ratio >= requirement.minimum };
  });
}

/** The requirements a theme fails. Empty means the theme meets WCAG AA. */
export function contrastFailures(theme: Theme): ContrastResult[] {
  return measureContrast(theme).filter((result) => !result.passes);
}
