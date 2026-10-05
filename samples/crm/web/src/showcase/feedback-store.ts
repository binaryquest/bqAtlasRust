import { computed, signal } from "@angular/core";
export interface ImportPreviewRow {
  code: string;
  name: string;
  quantity: number;
}
const initialRows = (): ImportPreviewRow[] => [
  { code: "P-100", name: "Office desk", quantity: 12 },
  { code: "P-200", name: "Task chair", quantity: 24 },
  { code: "P-300", name: "Storage cabinet", quantity: 8 },
];
export function feedbackDelay(signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) {
      reject(new Error("Cancelled"));
      return;
    }
    const abort = () => {
      clearTimeout(timer);
      reject(new Error("Cancelled"));
    };
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", abort);
      resolve();
    }, 650);
    signal.addEventListener("abort", abort, { once: true });
  });
}
/** Local import simulation. Each task owns its rows, cancellation and result. */
export class FeedbackDemoStore {
  readonly name = signal("September inventory");
  readonly rows = signal(initialRows());
  readonly phase = signal<
    "idle" | "validating" | "importing" | "complete" | "error" | "cancelled"
  >("idle");
  readonly completed = signal(0);
  readonly failNext = signal(false);
  readonly error = signal("");
  readonly imported = signal<ImportPreviewRow[]>([]);
  readonly busy = computed(() =>
    ["validating", "importing"].includes(this.phase()),
  );
  readonly message = computed(
    () =>
      ({
        idle: "Ready to validate the sample.",
        validating: "Validating sample rows…",
        importing: `Importing ${this.completed()} of ${this.rows().length} rows…`,
        complete: `Imported ${this.imported().length} sample rows locally.`,
        error: this.error(),
        cancelled: "Import cancelled. Preview rows are retained.",
      })[this.phase()],
  );
  private controller?: AbortController;
  private disposed = false;
  rename(value: string) {
    if (this.busy() || this.disposed) return;
    const name = value.trim();
    if (!name || name.length > 80)
      throw new Error("Enter a batch name of 1–80 characters.");
    this.name.set(name);
  }
  reset() {
    if (this.disposed) return;
    this.controller?.abort();
    this.controller = undefined;
    this.phase.set("idle");
    this.completed.set(0);
    this.rows.set(initialRows());
    this.imported.set([]);
    this.error.set("");
  }
  cancel() {
    if (!this.busy()) return;
    this.controller?.abort();
    this.controller = undefined;
    this.phase.set("cancelled");
  }
  async start(wait: (signal: AbortSignal) => Promise<void> = feedbackDelay) {
    if (this.busy() || this.disposed) return;
    const controller = new AbortController();
    this.controller = controller;
    const rows = structuredClone(this.rows()),
      fail = this.failNext();
    this.failNext.set(false);
    this.error.set("");
    this.completed.set(0);
    this.phase.set("validating");
    const current = () =>
      this.controller === controller &&
      !controller.signal.aborted &&
      !this.disposed;
    try {
      await wait(controller.signal);
      if (!current()) return;
      if (
        !rows.length ||
        rows.some(
          (row) =>
            !row.code.trim() ||
            !Number.isInteger(row.quantity) ||
            row.quantity < 0,
        ) ||
        new Set(rows.map((row) => row.code)).size !== rows.length
      )
        throw new Error(
          "Preview rows must have unique codes and non-negative whole quantities.",
        );
      this.phase.set("importing");
      for (let index = 0; index < rows.length; index++) {
        await wait(controller.signal);
        if (!current()) return;
        if (fail && index === 1)
          throw new Error(
            "Simulated connection failure. Nothing was committed; retry the retained preview.",
          );
        this.completed.set(index + 1);
      }
      this.imported.set(rows);
      this.phase.set("complete");
    } catch (error) {
      if (current()) {
        this.error.set(
          error instanceof Error ? error.message : "Import failed.",
        );
        this.phase.set("error");
      }
    }
  }
  dispose() {
    this.disposed = true;
    this.controller?.abort();
    this.controller = undefined;
  }
}
