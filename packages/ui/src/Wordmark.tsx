export interface WordmarkProps {
  /** Product name shown after the Vainnor mark. Omit for the bare mark. */
  name?: string;
}

/** The Vainnor `^v█` mark, optionally followed by a product name. */
export function Wordmark({ name }: WordmarkProps) {
  return (
    <span className="t-wordmark" role="img" aria-label={name ? `Vainnor ${name}` : "Vainnor"}>
      <span className="t-wordmark__mark" aria-hidden="true">
        ^v<span className="t-wordmark__cursor">█</span>
      </span>
      {name ? (
        <span className="t-wordmark__name" aria-hidden="true">
          {name}
        </span>
      ) : null}
    </span>
  );
}
