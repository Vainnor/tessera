import type { APIRoute } from "astro";

import license from "../../../../LICENSE?raw";

// Serves the repository's LICENSE verbatim, so the site and the repo cannot disagree.
export const GET: APIRoute = () =>
  new Response(license, { headers: { "Content-Type": "text/plain; charset=utf-8" } });
