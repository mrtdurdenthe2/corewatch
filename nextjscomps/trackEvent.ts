 "use server";
 
 export type TrackEventInput = {
   event: string;
   url?: string;
   referrer?: string;
 };
 
 function assertValidEventInput(input: TrackEventInput) {
   // Keep this strict to reduce abuse + avoid log/DB garbage.
   if (!input || typeof input !== "object") throw new Error("Invalid input");
 
   if (typeof input.event !== "string") throw new Error("Invalid event");
   const event = input.event.trim();
   if (event.length < 1 || event.length > 64) throw new Error("Invalid event");
 
   if (input.url != null) {
     if (typeof input.url !== "string") throw new Error("Invalid url");
     if (input.url.length > 2048) throw new Error("Invalid url");
   }
 
   if (input.referrer != null) {
     if (typeof input.referrer !== "string") throw new Error("Invalid referrer");
     if (input.referrer.length > 2048) throw new Error("Invalid referrer");
   }
 }
 
 /**
  * Server Action: call this from a Client Component on click/view/etc.
  *
  * Flow:
  * Browser -> Next.js Server Action (this file) -> Corewatch ingest server (Rust)
  *
  * Env:
  * - COREWATCH_INGEST_SECRET: required (server-only)
  * - COREWATCH_INGEST_URL: optional, defaults to http://127.0.0.1:3000/event
  */
 export async function trackEvent(input: TrackEventInput): Promise<void> {
   assertValidEventInput(input);
 
   const secret = process.env.COREWATCH_INGEST_SECRET;
   if (!secret) throw new Error("COREWATCH_INGEST_SECRET is not set");
 
   const ingestBase =
    process.env.COREWATCH_INGEST_URL ?? "http://127.0.0.1:6767/event";
 
   // Your Rust server currently expects GET /event with query params.
   // If you later switch Rust to POST JSON, update this accordingly.
   const url = new URL(ingestBase);
   url.searchParams.set("event", input.event.trim());
   if (input.url) url.searchParams.set("url", input.url);
   if (input.referrer) url.searchParams.set("referrer", input.referrer);
 
   let res: Response;
   try {
     res = await fetch(url, {
       method: "GET",
       headers: {
         // NOTE: Rust must verify this header for it to be effective.
         authorization: `Bearer ${secret}`,
       },
       cache: "no-store",
     });
   } catch (e) {
     const message = e instanceof Error ? e.message : String(e);
     throw new Error(`Corewatch request failed: ${message}`);
   }
 
   if (!res.ok) {
     // Avoid leaking anything sensitive; just return status.
     throw new Error(`Corewatch ingest failed: ${res.status}`);
   }
 }

