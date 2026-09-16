import type { RequestHandler } from "./$types";

export const GET: RequestHandler = () => {
  return new Response(JSON.stringify({ status: "ok" }), {
    headers: {
      "content-type": "application/json",
    },
  });
};
