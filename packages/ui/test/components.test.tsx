import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Button, Wordmark } from "../src/index.ts";

describe("Wordmark", () => {
  it("names the product for assistive tech and hides the glyphs", () => {
    const html = renderToStaticMarkup(<Wordmark name="tessera" />);
    expect(html).toContain('role="img"');
    expect(html).toContain('aria-label="Vainnor tessera"');
    expect(html).toContain('<span class="t-wordmark__mark" aria-hidden="true">^v<span class="t-wordmark__cursor">█</span></span>');
  });

  it("renders the bare mark", () => {
    const html = renderToStaticMarkup(<Wordmark />);
    expect(html).toContain('aria-label="Vainnor"');
    expect(html).not.toContain("t-wordmark__name");
  });
});

describe("Button", () => {
  it("defaults to a non-submitting secondary button", () => {
    expect(renderToStaticMarkup(<Button>Save</Button>)).toBe(
      '<button type="button" class="t-button">Save</button>',
    );
  });

  it("applies the primary variant and keeps extra classes", () => {
    const html = renderToStaticMarkup(
      <Button variant="primary" className="wide">
        Go
      </Button>,
    );
    expect(html).toContain('class="t-button t-button--primary wide"');
  });
});
