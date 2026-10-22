import { contrastFailures, type ContrastResult } from "./contrast.ts";
import {
  COLOR_ROLES,
  DENSITIES,
  ELEVATIONS,
  FONT_FAMILIES,
  HEADING_CASES,
  MODES,
  type ColorRole,
  type Theme,
} from "./types.ts";

/** Imported theme files larger than this are refused before they are parsed. */
export const MAX_THEME_BYTES = 16 * 1024;

/** Preset ids. A user theme may not reuse one, so importing cannot shadow a built-in. */
export const BUILT_IN_THEME_IDS = ["vainnor", "light", "dark", "paper", "high-contrast"] as const;

export type ValidationResult =
  { ok: true; theme: Theme; contrastWarnings: ContrastResult[] } | { ok: false; errors: string[] };

const HEX = /^#[0-9a-fA-F]{6}$/;
const ID = /^[a-z0-9-]{1,40}$/;
// eslint-disable-next-line no-control-regex -- the point is to reject control characters
const CONTROL = /[\u0000-\u001f\u007f]/;
const TOP_LEVEL_KEYS = [
  "$schema",
  "schemaVersion",
  "id",
  "name",
  "description",
  "mode",
  "colors",
  "typography",
  "radius",
  "density",
  "elevation",
  "motion",
] as const;
const TYPOGRAPHY_KEYS = ["body", "heading", "scale", "headingCase"] as const;

type Plain = Record<string, unknown>;

const isPlainObject = (value: unknown): value is Plain =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isOneOf = <T extends string>(options: readonly T[], value: unknown): value is T =>
  typeof value === "string" && (options as readonly string[]).includes(value);

function unknownKeys(value: Plain, allowed: readonly string[], at: string): string[] {
  return Object.keys(value)
    .filter((key) => !allowed.includes(key))
    .map((key) => `${at}: unknown key ${JSON.stringify(key.slice(0, 40))}`);
}

function text(value: unknown, at: string, max: number, errors: string[]): string | undefined {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.length > max ||
    CONTROL.test(value)
  ) {
    errors.push(`${at}: must be 1 to ${max} characters with no control characters`);
    return undefined;
  }
  return value;
}

/**
 * Parses and validates the text of an imported theme file.
 *
 * Order matters: the size limit runs before `JSON.parse`, and the result is a freshly built
 * object, so nothing from the file except checked values reaches the caller.
 */
export function parseTheme(source: string): ValidationResult {
  if (new TextEncoder().encode(source).byteLength > MAX_THEME_BYTES) {
    return { ok: false, errors: [`theme files are limited to ${MAX_THEME_BYTES} bytes`] };
  }
  let value: unknown;
  try {
    value = JSON.parse(source);
  } catch {
    return { ok: false, errors: ["not valid JSON"] };
  }
  return validateTheme(value);
}

/**
 * Validates an already-parsed theme. Structural problems are errors; contrast below WCAG AA
 * is reported as warnings, so a user can still choose a low-contrast theme knowingly.
 */
export function validateTheme(
  value: unknown,
  options: { allowBuiltInId?: boolean } = {},
): ValidationResult {
  const errors: string[] = [];
  if (!isPlainObject(value)) {
    return { ok: false, errors: ["a theme must be a JSON object"] };
  }
  errors.push(...unknownKeys(value, TOP_LEVEL_KEYS, "theme"));

  if (value.schemaVersion !== 1) errors.push("schemaVersion: must be 1");
  if (
    value.$schema !== undefined &&
    (typeof value.$schema !== "string" || value.$schema.length > 200)
  ) {
    errors.push("$schema: must be a string of at most 200 characters");
  }

  const id = typeof value.id === "string" && ID.test(value.id) ? value.id : undefined;
  if (id === undefined) errors.push("id: must be 1 to 40 lowercase letters, digits or hyphens");
  else if (!options.allowBuiltInId && (BUILT_IN_THEME_IDS as readonly string[]).includes(id)) {
    errors.push(`id: "${id}" belongs to a built-in theme`);
  }
  const name = text(value.name, "name", 40, errors);
  const description =
    value.description === undefined
      ? undefined
      : text(value.description, "description", 200, errors);
  if (!isOneOf(MODES, value.mode)) errors.push(`mode: must be one of ${MODES.join(", ")}`);

  const colors = {} as Record<ColorRole, string>;
  if (isPlainObject(value.colors)) {
    errors.push(...unknownKeys(value.colors, COLOR_ROLES, "colors"));
    for (const role of COLOR_ROLES) {
      const color = value.colors[role];
      if (typeof color === "string" && HEX.test(color)) colors[role] = color.toLowerCase();
      else errors.push(`colors.${role}: must be a #rrggbb color`);
    }
  } else {
    errors.push("colors: must be an object");
  }

  const typography = isPlainObject(value.typography) ? value.typography : undefined;
  if (typography === undefined) {
    errors.push("typography: must be an object");
  } else {
    errors.push(...unknownKeys(typography, TYPOGRAPHY_KEYS, "typography"));
    for (const key of ["body", "heading"] as const) {
      if (!isOneOf(FONT_FAMILIES, typography[key])) {
        errors.push(`typography.${key}: must be one of ${FONT_FAMILIES.join(", ")}`);
      }
    }
    const scale = typography.scale;
    if (typeof scale !== "number" || !Number.isFinite(scale) || scale < 0.875 || scale > 1.25) {
      errors.push("typography.scale: must be a number from 0.875 to 1.25");
    }
    if (!isOneOf(HEADING_CASES, typography.headingCase)) {
      errors.push(`typography.headingCase: must be one of ${HEADING_CASES.join(", ")}`);
    }
  }

  const radius = value.radius;
  if (typeof radius !== "number" || !Number.isInteger(radius) || radius < 0 || radius > 16) {
    errors.push("radius: must be a whole number from 0 to 16");
  }
  if (!isOneOf(DENSITIES, value.density))
    errors.push(`density: must be one of ${DENSITIES.join(", ")}`);
  if (!isOneOf(ELEVATIONS, value.elevation)) {
    errors.push(`elevation: must be one of ${ELEVATIONS.join(", ")}`);
  }
  if (typeof value.motion !== "boolean") errors.push("motion: must be true or false");

  if (errors.length > 0) return { ok: false, errors };

  // Every field below was checked above; build a new object rather than returning the input.
  const checked = typography as Plain;
  const theme: Theme = {
    schemaVersion: 1,
    id: id as string,
    name: name as string,
    ...(description === undefined ? {} : { description }),
    mode: value.mode as Theme["mode"],
    colors,
    typography: {
      body: checked.body as Theme["typography"]["body"],
      heading: checked.heading as Theme["typography"]["heading"],
      scale: checked.scale as number,
      headingCase: checked.headingCase as Theme["typography"]["headingCase"],
    },
    radius: radius as number,
    density: value.density as Theme["density"],
    elevation: value.elevation as Theme["elevation"],
    motion: value.motion as boolean,
  };
  return { ok: true, theme, contrastWarnings: contrastFailures(theme) };
}
