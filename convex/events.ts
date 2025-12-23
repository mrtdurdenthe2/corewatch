import { mutation } from "./_generated/server";
import { v } from "convex/values";

// Public mutation: write a single analytics event.
// - Validates args
// - Sets receivedAtMs server-side
export const addEvent = mutation({
  args: {
    event: v.string(),
    url: v.optional(v.string()),
    referrer: v.optional(v.string()),
    userAgent: v.optional(v.string()),
    ip: v.optional(v.string()),
  },
  handler: async (ctx, args) => {
    const event = args.event.trim();
    if (event.length < 1 || event.length > 64) {
      throw new Error("Invalid event");
    }

    // Basic bounds to reduce garbage / abuse. Keep aligned with your ingest limits.
    const url = args.url?.slice(0, 2048);
    const referrer = args.referrer?.slice(0, 2048);
    const userAgent = args.userAgent?.slice(0, 512);
    const ip = args.ip?.slice(0, 128);

    return await ctx.db.insert("events", {
      event,
      url,
      referrer,
      userAgent,
      ip,
      receivedAtMs: Date.now(),
    });
  },
});


