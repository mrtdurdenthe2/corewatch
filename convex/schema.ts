import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

// Stores analytics events ingested by the Rust server (/event).
// Rust currently receives:
// - query params: event, url?, referrer?
// - headers: user-agent?, authorization (not stored)
// - client IP (from axum-client-ip)
// - server timestamp (added on receipt)
export default defineSchema({
  events: defineTable({
    // Query params
    event: v.string(), // e.g. "click", "view"
    url: v.optional(v.string()),
    referrer: v.optional(v.string()),

    // Request metadata (optional because it can be absent or stripped)
    userAgent: v.optional(v.string()),
    ip: v.optional(v.string()),

    // Server-side receipt time (milliseconds since epoch).
    // Note: Convex also provides _creationTime automatically; we keep this if
    // you want an explicit "receivedAt" field from the ingest pipeline.
    receivedAtMs: v.number(),
  })
    .index("by_event", ["event"])
    .index("by_receivedAtMs", ["receivedAtMs"])
    .index("by_ip", ["ip"])
    .index("by_url", ["url"]),
});

