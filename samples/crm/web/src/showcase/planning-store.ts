import { computed, signal } from "@angular/core";
import { atlasDate, atlasRichText } from "@bqatlas/ui";
import type { AtlasRichDocument } from "@bqatlas/ui";
export interface PlanningDraft {
  title: string;
  date: string;
  notes: AtlasRichDocument;
}
export function planningSeed(): PlanningDraft {
  return {
    title: "Dispatch customer orders",
    date: "2026-09-28",
    notes: {
      blocks: [
        { id: "intro", kind: "heading", text: "Dispatch instructions" },
        {
          id: "note",
          kind: "bullet",
          text: "Confirm quantities with the warehouse before loading.",
        },
      ],
    },
  };
}
export class PlanningStore {
  readonly draft = signal(planningSeed());
  readonly saved = signal(planningSeed());
  readonly saving = signal(false);
  readonly readonly = signal(false);
  readonly failNext = signal(false);
  readonly error = signal("");
  readonly message = signal("");
  readonly dirty = computed(
    () => JSON.stringify(this.draft()) !== JSON.stringify(this.saved()),
  );
  private generation = 0;
  private disposed = false;
  update(patch: Partial<PlanningDraft>) {
    if (this.saving() || this.readonly() || this.disposed) return;
    this.draft.update((value) => ({ ...value, ...patch }));
    this.error.set("");
  }
  reset() {
    if (this.disposed) return;
    this.generation++;
    this.saving.set(false);
    this.draft.set(structuredClone(this.saved()));
    this.error.set("");
    this.message.set("Draft restored to the last local save.");
  }
  async save(
    wait: () => Promise<void> = () =>
      new Promise((resolve) => setTimeout(resolve, 350)),
  ) {
    if (this.saving()) throw new Error("A save is already in progress.");
    if (this.readonly() || this.disposed)
      throw new Error("This plan cannot be edited.");
    const snapshot = structuredClone(this.draft());
    let dateValid = true;
    try {
      atlasDate(snapshot.date);
    } catch {
      dateValid = false;
    }
    if (
      !snapshot.title.trim() ||
      !dateValid ||
      !atlasRichText(snapshot.notes).trim()
    ) {
      this.error.set(
        "Enter a title, a valid delivery date and at least one note.",
      );
      throw new Error(this.error());
    }
    const generation = ++this.generation,
      fail = this.failNext();
    this.failNext.set(false);
    this.saving.set(true);
    this.error.set("");
    try {
      await wait();
      if (generation !== this.generation || this.disposed || this.readonly())
        throw new Error("Save cancelled. The draft was not committed.");
      if (fail)
        throw new Error(
          "Simulated save failure. Your draft is retained; try saving again.",
        );
      this.saved.set(snapshot);
      this.message.set("Saved locally in this example.");
    } catch (error) {
      if (generation === this.generation && !this.disposed)
        this.error.set(error instanceof Error ? error.message : "Save failed.");
      throw error;
    } finally {
      if (generation === this.generation) this.saving.set(false);
    }
  }
  dispose() {
    this.disposed = true;
    this.generation++;
  }
}
