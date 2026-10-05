import { computed, signal } from "@angular/core";
import { parseDecimalUnits } from "@bqatlas/contracts";
import { atlasCommandAllowed } from "@bqatlas/ui";
import type { AtlasCommand } from "@bqatlas/ui";
const seed = () => ({
  title: "Autumn wholesale · USD",
  desk: "420.00",
  chair: "185.50",
  warehouses: ["dhaka", "chattogram"],
});
/** Independent local drafts. This adapter never calls an application API. */
export class CommandDemoStore {
  readonly saved = signal(seed());
  readonly draft = signal(seed());
  readonly manage = signal(true);
  readonly readonly = signal(false);
  readonly saving = signal(false);
  readonly failNext = signal(false);
  readonly error = signal("");
  readonly message = signal("");
  readonly dirty = computed(
    () => JSON.stringify(this.draft()) !== JSON.stringify(this.saved()),
  );
  readonly permissions = computed(() =>
    this.manage() ? ["prices.manage", "prices.export"] : ["prices.export"],
  );
  readonly commands = computed<AtlasCommand[]>(() => [
    {
      id: "save",
      label: "Save draft",
      icon: "save",
      primary: true,
      permission: "prices.manage",
      disabled: !this.dirty() || this.readonly() || this.saving(),
      group: "write",
    },
    {
      id: "save-copy",
      label: "Save a copy",
      icon: "plus",
      permission: "prices.manage",
      disabled: this.readonly() || this.saving(),
      group: "write",
    },
    {
      id: "restore",
      label: "Revert changes",
      icon: "back",
      permission: "prices.manage",
      disabled: !this.dirty() || this.readonly() || this.saving(),
      group: "write",
    },
    {
      id: "preview",
      label: "Preview prices",
      icon: "search",
      disabled: this.saving(),
      group: "read",
    },
    {
      id: "export",
      label: "Preview export",
      icon: "orders",
      permission: "prices.export",
      disabled: this.saving(),
      group: "read",
    },
    {
      id: "history",
      label: "View activity",
      icon: "activity",
      disabled: this.saving(),
      group: "read",
    },
  ]);
  readonly history = signal<string[]>(["Sample price list opened."]);
  private generation = 0;
  dispose() {
    this.generation++;
  }
  reset() {
    this.generation++;
    this.saved.set(seed());
    this.draft.set(seed());
    this.saving.set(false);
    this.failNext.set(false);
    this.error.set("");
    this.message.set("");
    this.history.set(["Sample price list opened."]);
  }
  async save(copy = false) {
    if (this.saving() || this.readonly() || !this.manage())
      throw new Error("Editing permission is unavailable.");
    const snapshot = structuredClone(this.draft());
    if (!snapshot.title.trim()) throw new Error("Enter a price-list name.");
    if (
      [snapshot.desk, snapshot.chair].some(
        (value) => parseDecimalUnits(value, 2, "1000000") === null,
      )
    )
      throw new Error(
        "Prices must be valid USD amounts with at most two decimal places.",
      );
    if (!snapshot.warehouses.length)
      throw new Error("Assign at least one warehouse.");
    const generation = ++this.generation,
      fail = this.failNext();
    this.failNext.set(false);
    this.saving.set(true);
    this.error.set("");
    try {
      await new Promise((resolve) => setTimeout(resolve, 350));
      if (generation !== this.generation) throw new Error("Save cancelled.");
      if (!this.manage() || this.readonly())
        throw new Error("Editing permission changed. Your draft is retained.");
      if (fail)
        throw new Error(
          "Simulated save failure. Your draft is retained; try Save draft again.",
        );
      const unchanged =
        JSON.stringify(this.draft()) === JSON.stringify(snapshot);
      if (copy) snapshot.title += " · copy";
      this.saved.set(snapshot);
      if (unchanged) this.draft.set(structuredClone(snapshot));
      this.message.set(copy ? "Saved as a local copy." : "Local draft saved.");
      this.history.update((history) => [...history, this.message()]);
    } finally {
      if (generation === this.generation) this.saving.set(false);
    }
  }
  async execute(id: string) {
    if (!atlasCommandAllowed(this.commands(), id, this.permissions()))
      throw new Error("This command is unavailable.");
    this.error.set("");
    try {
      if (id === "save" || id === "save-copy")
        await this.save(id === "save-copy");
      else if (id === "restore") {
        this.draft.set(structuredClone(this.saved()));
        this.message.set("Changes reverted.");
      } else if (id === "preview")
        this.message.set(
          `${this.draft().title}: desk ${this.draft().desk} USD, chair ${this.draft().chair} USD.`,
        );
      else if (id === "export")
        this.message.set(
          `Export preview: ${this.draft().warehouses.length} warehouses, 2 prices. Nothing was downloaded or sent.`,
        );
      else this.message.set(this.history().join(" "));
    } catch (error) {
      this.error.set(
        error instanceof Error ? error.message : "Command failed.",
      );
      throw error;
    }
  }
}
