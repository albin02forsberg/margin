import { inactiveStamp } from "../clock.ts";
import type { Package } from "../commands.ts";

/** Space i D: an org inactive timestamp for now at the cursor (replacing a selection). */
export default {
  id: "insert-timestamp",
  load: (margin) => margin.command({ label: "Insert timestamp now (inactive)", leader: "i D", run: () => !!margin.buffer.path && margin.buffer.insert(inactiveStamp(new Date())) }), // no-op outside a note, as before
} satisfies Package;
