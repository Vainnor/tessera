import type { ButtonHTMLAttributes } from "react";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary";
}

/** A button that defaults to `type="button"`, so it never submits a form by accident. */
export function Button({ variant = "secondary", className, type = "button", ...rest }: ButtonProps) {
  const classes = ["t-button", variant === "primary" ? "t-button--primary" : "", className ?? ""]
    .filter(Boolean)
    .join(" ");
  return <button type={type} className={classes} {...rest} />;
}
